use std::ffi::CString;
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::mem::size_of;
use std::os::fd::{FromRawFd, RawFd};
use std::time::{Duration, Instant};

use crate::SkillError;

const MAX_PACKET: usize = 64 * 1024;
const MAX_BULK: usize = 64 * 1024 * 1024;
const BULK_MARKER: &[u8] = b"\0EOS-SKILL-BULK1";

pub(crate) fn send_packet(fd: RawFd, bytes: &[u8]) -> Result<(), SkillError> {
  send_packet_with_timeout(fd, bytes, Duration::from_secs(5))
}

fn bounded_send(
  fd: RawFd,
  expected: usize,
  timeout: Duration,
  mut operation: impl FnMut() -> isize,
) -> Result<(), SkillError> {
  let deadline = Instant::now() + timeout;
  loop {
    let written = operation();
    if written == expected as isize {
      return Ok(());
    }
    if written >= 0 {
      return Err(SkillError::Protocol("worker_protocol_io"));
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::Interrupted {
      continue;
    }
    if error.kind() != io::ErrorKind::WouldBlock {
      return Err(SkillError::Protocol("worker_protocol_io"));
    }
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
      return Err(SkillError::Protocol("worker_timeout"));
    }
    let mut poll = libc::pollfd {
      fd,
      events: libc::POLLOUT,
      revents: 0,
    };
    let wait = remaining.as_millis().clamp(1, i32::MAX as u128) as i32;
    let ready = unsafe { libc::poll(&mut poll, 1, wait) };
    if ready == 0 {
      return Err(SkillError::Protocol("worker_timeout"));
    }
    if ready < 0 && io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
      return Err(SkillError::Protocol("worker_protocol_io"));
    }
  }
}

fn send_packet_with_timeout(fd: RawFd, bytes: &[u8], timeout: Duration) -> Result<(), SkillError> {
  if bytes.len() > MAX_BULK {
    return Err(SkillError::Protocol("worker_packet_oversize"));
  }
  if bytes.len() <= MAX_PACKET {
    return bounded_send(fd, bytes.len(), timeout, || unsafe {
      libc::send(
        fd,
        bytes.as_ptr().cast(),
        bytes.len(),
        libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT,
      )
    });
  }
  let label = CString::new("eos-skill-packet").expect("static label");
  let bulk_fd =
    unsafe { libc::memfd_create(label.as_ptr(), libc::MFD_CLOEXEC | libc::MFD_ALLOW_SEALING) };
  if bulk_fd < 0 {
    return Err(SkillError::Protocol("worker_bulk_unavailable"));
  }
  let mut bulk = unsafe { File::from_raw_fd(bulk_fd) };
  bulk
    .write_all(bytes)
    .map_err(|_| SkillError::Protocol("worker_bulk_unavailable"))?;
  bulk
    .seek(SeekFrom::Start(0))
    .map_err(|_| SkillError::Protocol("worker_bulk_unavailable"))?;
  let seals = libc::F_SEAL_SEAL | libc::F_SEAL_SHRINK | libc::F_SEAL_GROW | libc::F_SEAL_WRITE;
  if unsafe { libc::fcntl(bulk_fd, libc::F_ADD_SEALS, seals) } < 0 {
    return Err(SkillError::Protocol("worker_bulk_unavailable"));
  }
  let mut header = Vec::with_capacity(BULK_MARKER.len() + 8);
  header.extend_from_slice(BULK_MARKER);
  header.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
  let mut iov = libc::iovec {
    iov_base: header.as_mut_ptr().cast(),
    iov_len: header.len(),
  };
  let mut control = vec![0u8; unsafe { libc::CMSG_SPACE(size_of::<RawFd>() as u32) } as usize];
  let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
  message.msg_iov = &mut iov;
  message.msg_iovlen = 1;
  message.msg_control = control.as_mut_ptr().cast();
  message.msg_controllen = control.len();
  let cmsg = unsafe { libc::CMSG_FIRSTHDR(&message) };
  if cmsg.is_null() {
    return Err(SkillError::Protocol("worker_bulk_unavailable"));
  }
  unsafe {
    (*cmsg).cmsg_level = libc::SOL_SOCKET;
    (*cmsg).cmsg_type = libc::SCM_RIGHTS;
    (*cmsg).cmsg_len = libc::CMSG_LEN(size_of::<RawFd>() as u32) as usize;
    std::ptr::write(libc::CMSG_DATA(cmsg).cast::<RawFd>(), bulk_fd);
  }
  bounded_send(fd, header.len(), timeout, || unsafe {
    libc::sendmsg(fd, &message, libc::MSG_NOSIGNAL | libc::MSG_DONTWAIT)
  })
}

pub(crate) fn recv_packet(fd: RawFd, timeout: Duration) -> Result<Vec<u8>, SkillError> {
  let milliseconds = timeout.as_millis().min(i32::MAX as u128) as i32;
  let mut poll = libc::pollfd {
    fd,
    events: libc::POLLIN,
    revents: 0,
  };
  if unsafe { libc::poll(&mut poll, 1, milliseconds) } <= 0 {
    return Err(SkillError::Protocol("worker_timeout"));
  }
  let mut bytes = vec![0u8; MAX_PACKET + 1];
  let mut iov = libc::iovec {
    iov_base: bytes.as_mut_ptr().cast(),
    iov_len: bytes.len(),
  };
  let mut control = vec![0u8; unsafe { libc::CMSG_SPACE(size_of::<RawFd>() as u32) } as usize];
  let mut message: libc::msghdr = unsafe { std::mem::zeroed() };
  message.msg_iov = &mut iov;
  message.msg_iovlen = 1;
  message.msg_control = control.as_mut_ptr().cast();
  message.msg_controllen = control.len();
  let length = unsafe { libc::recvmsg(fd, &mut message, libc::MSG_CMSG_CLOEXEC) };
  if length <= 0
    || length as usize > MAX_PACKET
    || message.msg_flags & (libc::MSG_TRUNC | libc::MSG_CTRUNC) != 0
  {
    return Err(SkillError::Protocol("worker_packet_invalid"));
  }
  bytes.truncate(length as usize);
  let cmsg = unsafe { libc::CMSG_FIRSTHDR(&message) };
  if bytes.starts_with(BULK_MARKER) {
    if bytes.len() != BULK_MARKER.len() + 8 || cmsg.is_null() {
      return Err(SkillError::Protocol("worker_packet_invalid"));
    }
    let expected = u64::from_le_bytes(
      bytes[BULK_MARKER.len()..]
        .try_into()
        .expect("length checked"),
    );
    if expected as usize > MAX_BULK || expected < MAX_PACKET as u64 + 1 {
      return Err(SkillError::Protocol("worker_packet_invalid"));
    }
    if unsafe {
      (*cmsg).cmsg_level != libc::SOL_SOCKET
        || (*cmsg).cmsg_type != libc::SCM_RIGHTS
        || (*cmsg).cmsg_len != libc::CMSG_LEN(size_of::<RawFd>() as u32) as usize
        || !libc::CMSG_NXTHDR(&message, cmsg).is_null()
    } {
      return Err(SkillError::Protocol("worker_packet_invalid"));
    }
    let bulk_fd = unsafe { std::ptr::read(libc::CMSG_DATA(cmsg).cast::<RawFd>()) };
    let bulk = unsafe { File::from_raw_fd(bulk_fd) };
    let seals = unsafe { libc::fcntl(bulk_fd, libc::F_GET_SEALS) };
    let required = libc::F_SEAL_SEAL | libc::F_SEAL_SHRINK | libc::F_SEAL_GROW | libc::F_SEAL_WRITE;
    if seals < 0
      || seals & required != required
      || bulk.metadata().map_or(true, |meta| meta.len() != expected)
    {
      return Err(SkillError::Protocol("worker_packet_invalid"));
    }
    let mut content = Vec::with_capacity(expected as usize);
    bulk
      .take(expected + 1)
      .read_to_end(&mut content)
      .map_err(|_| SkillError::Protocol("worker_packet_invalid"))?;
    if content.len() != expected as usize {
      return Err(SkillError::Protocol("worker_packet_invalid"));
    }
    return Ok(content);
  }
  if !cmsg.is_null() {
    return Err(SkillError::Protocol("worker_packet_invalid"));
  }
  Ok(bytes)
}

#[cfg(test)]
mod tests {
  use std::os::fd::{AsRawFd, FromRawFd};

  use super::*;

  #[test]
  fn inline_and_sealed_bulk_roundtrip() {
    let mut sockets = [0; 2];
    assert_eq!(
      unsafe {
        libc::socketpair(
          libc::AF_UNIX,
          libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
          0,
          sockets.as_mut_ptr(),
        )
      },
      0
    );
    let writer = unsafe { File::from_raw_fd(sockets[0]) };
    let reader = unsafe { File::from_raw_fd(sockets[1]) };
    for size in [1, MAX_PACKET, MAX_PACKET + 1, 1024 * 1024] {
      let bytes = vec![b'x'; size];
      send_packet(writer.as_raw_fd(), &bytes).unwrap();
      assert_eq!(
        recv_packet(reader.as_raw_fd(), Duration::from_secs(1)).unwrap(),
        bytes
      );
    }
  }

  #[test]
  fn oversized_packet_is_rejected() {
    let mut sockets = [0; 2];
    assert_eq!(
      unsafe {
        libc::socketpair(
          libc::AF_UNIX,
          libc::SOCK_SEQPACKET | libc::SOCK_CLOEXEC,
          0,
          sockets.as_mut_ptr(),
        )
      },
      0
    );
    let writer = unsafe { File::from_raw_fd(sockets[0]) };
    let _reader = unsafe { File::from_raw_fd(sockets[1]) };
    assert!(matches!(
      send_packet(writer.as_raw_fd(), &vec![0; MAX_BULK + 1]),
      Err(SkillError::Protocol("worker_packet_oversize"))
    ));
  }
}

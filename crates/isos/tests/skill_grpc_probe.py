#!/usr/bin/env python3
"""Invoke the selected Rust skill through the live EOS gRPC catalog."""

import sys
import time

import grpc
from google.protobuf.duration_pb2 import Duration
from google.protobuf.struct_pb2 import Struct
from island.generated.eos import skills_pb2_grpc
from island.generated.eos.skills import catalog_pb2, execution_pb2


def main():
    endpoint, revision, grant, phase = sys.argv[1:5]
    channel = grpc.insecure_channel(endpoint)
    grpc.channel_ready_future(channel).result(timeout=10)
    skills = skills_pb2_grpc.SkillServiceStub(channel)
    catalog = skills.ListSkills(catalog_pb2.ListSkillsRequest(), timeout=5)
    assert len(catalog.skills) == 1 and catalog.skills[0].reference.name == "base.move", catalog
    metadata = (("authorization", f"Bearer {grant}"),
                ("x-eos-active-revision", revision))
    def execute(mode, expected, reason=None):
        arguments = Struct()
        if mode:
            arguments.update({"mode": mode})
        started = skills.StartExecution(execution_pb2.StartExecutionRequest(
            reference=catalog_pb2.SkillReference(name="base.move", version="1.0.0"),
            arguments=arguments, request_id=f"owner-skill-{mode or 'success'}",
            timeout=Duration(seconds=8)), metadata=metadata, timeout=5)
        if mode == "wait":
            skills.TerminateExecution(execution_pb2.TerminateExecutionRequest(
                execution_id=started.execution_id), metadata=metadata, timeout=5)
        deadline = time.monotonic() + 10
        while time.monotonic() < deadline:
            execution = skills.GetExecution(
                execution_pb2.GetExecutionRequest(execution_id=started.execution_id),
                metadata=metadata, timeout=5)
            if execution.status not in (execution_pb2.EXECUTION_STATUS_QUEUED,
                                        execution_pb2.EXECUTION_STATUS_RUNNING):
                assert execution.status == expected, execution
                if reason is not None:
                    assert execution.reason_code == reason, execution
                print(f"selected skill {execution.execution_id}: {mode or 'success'} terminal={expected}")
                return
            time.sleep(.05)
        raise AssertionError(f"selected skill {mode} did not reach a terminal result")

    if phase == "after_restart":
        execute(None, execution_pb2.EXECUTION_STATUS_FAILED,
                "worker.primitive_denied")
        execute("fail", execution_pb2.EXECUTION_STATUS_FAILED)
        execute("wait", execution_pb2.EXECUTION_STATUS_CANCELLED)
    else:
        execute(None, execution_pb2.EXECUTION_STATUS_SUCCEEDED)


if __name__ == "__main__":
    main()

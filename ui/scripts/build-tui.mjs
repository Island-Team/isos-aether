import { build } from 'esbuild';
import { mkdir, writeFile } from 'node:fs/promises';
await mkdir('dist/tui', { recursive: true });
await build({ entryPoints: ['src/tui/index.tsx'], outfile: 'dist/tui/index.js', bundle: true, platform: 'node', format: 'esm', target: 'node22', sourcemap: false, external: ['./devtools.js'], banner: { js: "import { createRequire } from 'node:module'; const require = createRequire(import.meta.url);" } });
await writeFile('dist/tui/package.json', '{"type":"module"}\n');

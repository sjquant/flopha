import {mkdir, readFile, writeFile} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const currentDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(currentDir, '..');
const sourcePath = path.join(repoRoot, 'assets', 'brand', 'logo.svg');
const targetPath = path.join(repoRoot, 'website', 'static', 'img', 'logo.svg');

await mkdir(path.dirname(targetPath), {recursive: true});
const source = await readFile(sourcePath, 'utf8');
await writeFile(targetPath, source);

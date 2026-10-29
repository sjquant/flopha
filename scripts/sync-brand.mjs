import {mkdir, readFile, writeFile} from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

const currentDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(currentDir, '..');
const brandAssets = [
  ['favicon.svg', 'favicon.svg'],
  ['flopha-icon.svg', 'flopha-icon.svg'],
  ['flopha-icon-dark.svg', 'flopha-icon-dark.svg'],
  ['flopha-social-preview.png', 'flopha-social-preview.png'],
];

const targetDir = path.join(repoRoot, 'website', 'static', 'img');

await mkdir(targetDir, {recursive: true});

for (const [sourceName, targetName] of brandAssets) {
  const sourcePath = path.join(repoRoot, 'assets', 'brand', sourceName);
  const targetPath = path.join(targetDir, targetName);
  const source = await readFile(sourcePath);

  await writeFile(targetPath, source);
}

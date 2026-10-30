// Sets the app version everywhere it lives: npm run version:set -- 0.2.0
// The updater compares this number with latest.json, so all four must agree with the git tag.
import { execSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';

const version = process.argv[2];
if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version ?? '')) {
  console.error('usage: npm run version:set -- <major.minor.patch>');
  process.exit(1);
}

// Replaces only the first "version" field so the files keep their formatting.
const json = (path) => {
  const text = readFileSync(path, 'utf8');
  writeFileSync(path, text.replace(/("version":\s*")[^"]+(")/, `$1${version}$2`));
};
json('package.json');
json('apps/ui/package.json');
json('src-tauri/tauri.conf.json');

const cargo = readFileSync('Cargo.toml', 'utf8');
const next = cargo.replace(/(\[workspace\.package\][^[]*?\nversion = ")[^"]+(")/, `$1${version}$2`);
if (next === cargo && !cargo.includes(`version = "${version}"`)) {
  console.error('could not find version under [workspace.package] in Cargo.toml');
  process.exit(1);
}
writeFileSync('Cargo.toml', next);

// Keep the lockfiles in step (npm workspaces record their own version; Cargo.lock records the crates').
execSync('npm install --package-lock-only --ignore-scripts', { stdio: 'inherit' });
execSync('cargo update --workspace', { stdio: 'inherit' });

console.log(`version set to ${version}; next: git commit, git tag v${version}, git push --tags`);

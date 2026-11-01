const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const fs = require('node:fs');
const path = require('node:path');

const root = path.resolve(__dirname, '../..');
const sizes = [16, 24, 32, 48, 64, 128, 256, 512];

function readPng(filename) {
  const bytes = fs.readFileSync(filename);
  assert.equal(bytes.subarray(0, 8).toString('hex'), '89504e470d0a1a0a', `${filename} is a PNG`);
  return {
    bytes,
    width: bytes.readUInt32BE(16),
    height: bytes.readUInt32BE(20),
  };
}

for (const size of sizes) {
  const relative = `assets/icons/${size}x${size}.png`;
  const image = readPng(path.join(root, relative));
  assert.deepEqual([image.width, image.height], [size, size], `${relative} dimensions`);
  const tauri = readPng(path.join(root, 'crates/lianli-gui/src-tauri/icons', `${size}x${size}.png`));
  assert.equal(crypto.createHash('sha256').update(tauri.bytes).digest('hex'), crypto.createHash('sha256').update(image.bytes).digest('hex'), `${relative} Tauri copy`);
}

for (const [alias, size] of [['assets/icons/icon.png', 512], ['assets/icons/128x128@2x.png', 256]]) {
  const image = readPng(path.join(root, alias));
  assert.deepEqual([image.width, image.height], [size, size], `${alias} dimensions`);
}

const tauri = JSON.parse(fs.readFileSync(path.join(root, 'crates/lianli-gui/src-tauri/tauri.conf.json'), 'utf8'));
for (const size of sizes) assert(tauri.bundle.icon.includes(`icons/${size}x${size}.png`), `Tauri bundle includes ${size}x${size}`);

const desktop = fs.readFileSync(path.join(root, 'packaging/desktop/com.sgtaziz.lianlilinux.desktop'), 'utf8');
assert.match(desktop, /^Icon=com\.sgtaziz\.lianlilinux$/m);
for (const relative of ['debian/rules', 'packaging/fedora/lian-li-linux.spec', 'packaging/archlinux/git/PKGBUILD', 'packaging/archlinux/release/PKGBUILD']) {
  const recipe = fs.readFileSync(path.join(root, relative), 'utf8');
  assert.match(recipe, /for size in 16 24 32 48 64 128 256 512/,
    `${relative} packages all application icon sizes`);
  assert(
    recipe.includes('hicolor/$${size}x$${size}/apps/com.sgtaziz.lianlilinux.png') ||
      recipe.includes('hicolor/${size}x${size}/apps/com.sgtaziz.lianlilinux.png'),
    `${relative} installs icons under the desktop entry name`,
  );
}

const logo = 'prism-linux-control-center-logo.png';
const logoBytes = fs.readFileSync(path.join(root, 'assets/branding', logo));
assert.equal(crypto.createHash('sha256').update(logoBytes).digest('hex'),
  crypto.createHash('sha256').update(fs.readFileSync(path.join(root, 'crates/lianli-gui/src/assets/branding', logo))).digest('hex'),
  'About image matches the README logo asset');
assert(fs.readFileSync(path.join(root, 'README.md'), 'utf8').includes(`assets/branding/${logo}`));
assert(fs.readFileSync(path.join(root, 'crates/lianli-gui/src/views/SettingsPage.vue'), 'utf8').includes(`@/assets/branding/${logo}`));
const icon = 'prism-circuit-p-icon.png';
const iconBytes = fs.readFileSync(path.join(root, 'assets/branding', icon));
assert.equal(crypto.createHash('sha256').update(iconBytes).digest('hex'),
  crypto.createHash('sha256').update(fs.readFileSync(path.join(root, 'crates/lianli-gui/src/assets/branding', icon))).digest('hex'),
  'Sidebar image matches the extracted application icon');
assert(fs.readFileSync(path.join(root, 'crates/lianli-gui/src/components/layout/AppSidebar.vue'), 'utf8').includes(`@/assets/branding/${icon}`));

console.log(`Verified ${sizes.length} Linux/Tauri icon sizes, desktop package paths, and README/About/sidebar branding assets.`);

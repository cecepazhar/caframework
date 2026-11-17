import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT_DIR = path.resolve(__dirname, '..');
const REPO_ROOT = path.resolve(ROOT_DIR, '..');
const ROUTES_DIR = path.join(ROOT_DIR, 'src', 'routes');
const EXCLUDE_DIR = path.join(ROOT_DIR, '.svelte-kit-excluded');
const STASH_FILE = path.join(ROOT_DIR, '.svelte-kit-excluded-stash.json');

export function getEnabledModules() {
  const appTomlPath = process.env.CAF_APP_TOML || path.join(REPO_ROOT, 'app.toml');
  const modules = {
    profiles: true,
    ai: true,
    pro: true,
    feedback: true,
    updater: true,
    crash: true,
    integrations: true,
    scheduler: true,
    support_page: false,
    sample: false,
  };

  if (fs.existsSync(appTomlPath)) {
    const content = fs.readFileSync(appTomlPath, 'utf-8');
    const match = content.match(/\[modules\]([\s\S]*?)(?:\n\[|$)/);
    if (match) {
      for (const line of match[1].split('\n')) {
        const trimmed = line.trim();
        if (!trimmed || trimmed.startsWith('#')) continue;
        const [k, v] = trimmed.split('=').map((s) => s.trim());
        if (k && v !== undefined) {
          modules[k] = v === 'true';
        }
      }
    }
  }

  // Environment variable overrides (e.g. CAF_ENABLE_MOD_SAMPLE=1)
  for (const key of Object.keys(modules)) {
    const envKey = `CAF_ENABLE_MOD_${key.toUpperCase()}`;
    if (process.env[envKey] !== undefined) {
      modules[key] = process.env[envKey] === '1' || process.env[envKey] === 'true';
    }
  }
  return modules;
}

export function stashDisabledModules() {
  const modules = getEnabledModules();
  if (!fs.existsSync(ROUTES_DIR)) return;

  const entries = fs.readdirSync(ROUTES_DIR, { withFileTypes: true });
  const stashed = [];

  for (const entry of entries) {
    if (entry.isDirectory()) {
      const match = entry.name.match(/^\(mod-([a-zA-Z0-9_-]+)\)$/);
      if (match) {
        const modKey = match[1].replace(/-/g, '_');
        const isEnabled = modules[modKey] === true || modules[match[1]] === true;
        if (!isEnabled) {
          const srcPath = path.join(ROUTES_DIR, entry.name);
          const destDir = path.join(EXCLUDE_DIR);
          if (!fs.existsSync(destDir)) {
            fs.mkdirSync(destDir, { recursive: true });
          }
          const destPath = path.join(destDir, entry.name);
          if (fs.existsSync(destPath)) {
            fs.rmSync(destPath, { recursive: true, force: true });
          }
          fs.renameSync(srcPath, destPath);
          stashed.push({ name: entry.name, srcPath, destPath });
          console.log(`[CAF] Stashed disabled module route: ${entry.name}`);
        }
      }
    }
  }

  if (stashed.length > 0) {
    fs.writeFileSync(STASH_FILE, JSON.stringify(stashed, null, 2));
  }
}

export function restoreDisabledModules() {
  if (!fs.existsSync(STASH_FILE) && !fs.existsSync(EXCLUDE_DIR)) {
    return;
  }

  if (fs.existsSync(STASH_FILE)) {
    try {
      const stashed = JSON.parse(fs.readFileSync(STASH_FILE, 'utf-8'));
      for (const item of stashed) {
        if (fs.existsSync(item.destPath)) {
          if (fs.existsSync(item.srcPath)) {
            fs.rmSync(item.srcPath, { recursive: true, force: true });
          }
          fs.renameSync(item.destPath, item.srcPath);
          console.log(`[CAF] Restored module route: ${item.name}`);
        }
      }
    } catch (e) {
      console.error('[CAF] Failed to restore from stash file:', e);
    }
    fs.rmSync(STASH_FILE, { force: true });
  }

  // Also clean up any lingering folders in EXCLUDE_DIR
  if (fs.existsSync(EXCLUDE_DIR)) {
    const remaining = fs.readdirSync(EXCLUDE_DIR);
    for (const name of remaining) {
      const src = path.join(EXCLUDE_DIR, name);
      const dest = path.join(ROUTES_DIR, name);
      if (fs.existsSync(src)) {
        if (!fs.existsSync(dest)) {
          fs.renameSync(src, dest);
          console.log(`[CAF] Restored lingering module route: ${name}`);
        } else {
          fs.rmSync(src, { recursive: true, force: true });
        }
      }
    }
    fs.rmSync(EXCLUDE_DIR, { recursive: true, force: true });
  }
}

// CLI usage: node modules-exclusion.js stash | restore
const action = process.argv[2];
if (action === 'stash') {
  stashDisabledModules();
} else if (action === 'restore') {
  restoreDisabledModules();
}

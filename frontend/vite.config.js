import { defineConfig } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import process from "node:process";
import {
  getEnabledModules,
  stashDisabledModules,
  restoreDisabledModules,
} from "./scripts/modules-exclusion.js";

const host = process.env.TAURI_DEV_HOST;
const enabledModules = getEnabledModules();

function cafModuleExclusionPlugin() {
  return {
    name: "caf-module-exclusion",
    /** @param {any} config */
    configResolved(config) {
      if (config.command === "build") {
        stashDisabledModules();
      }
    },
    buildEnd() {
      restoreDisabledModules();
    },
    closeBundle() {
      restoreDisabledModules();
    },
  };
}

// Safety handlers so stashed routes are always restored
process.on("exit", () => {
  restoreDisabledModules();
});
process.on("SIGINT", () => {
  restoreDisabledModules();
  process.exit(130);
});
process.on("SIGTERM", () => {
  restoreDisabledModules();
  process.exit(143);
});

/** @type {Record<string, string>} */
const defines = {
  "import.meta.env.CAF_ENABLED_MODULES": JSON.stringify(enabledModules),
};
for (const [mod, val] of Object.entries(enabledModules)) {
  const normKey = mod.toUpperCase().replace(/-/g, "_");
  defines[`__CAF_MOD_${normKey}__`] = JSON.stringify(val);
}

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [cafModuleExclusionPlugin(), tailwindcss(), sveltekit()],
  define: defines,
  test: {
    environment: "jsdom",
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || "127.0.0.1",
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching the Rust crates
      ignored: ["**/crates/**"],
    },
  },
  build: {
    target: ["chrome61", "es2018"],
  },
}));

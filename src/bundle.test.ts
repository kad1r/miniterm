import { it, expect } from "vitest";
import config from "../src-tauri/tauri.conf.json";

// Global Constraint lock: this repo builds with the Windows GNU toolchain, which
// links WebView2Loader as an import library instead of statically. The compiled
// exe therefore needs WebView2Loader.dll beside it at runtime. Cargo drops a copy
// into the target directory, so a locally run build looks fine — only the
// installed app fails, with "WebView2Loader.dll was not found" and no build error
// anywhere. Bundling the DLL as a resource is what puts it next to the installed
// exe.
it("ships WebView2Loader.dll next to the installed executable", () => {
  expect(config.bundle.resources).toMatchObject({
    "resources/WebView2Loader.dll": "WebView2Loader.dll",
  });
});

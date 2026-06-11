#!/usr/bin/env node

const { spawn } = require("child_process");
const path = require("path");
const fs = require("fs");

const binaryName = process.platform === "win32" ? "ctgx.exe" : "ctgx";
const binaryPath = path.join(__dirname, binaryName);

if (!fs.existsSync(binaryPath)) {
  console.error(
    "Cottage binary not found. Please run: npm rebuild @sayanarijit/cottage"
  );
  process.exit(1);
}

const args = process.argv.slice(2);
const child = spawn(binaryPath, args, { stdio: "inherit" });

child.on("close", (code) => {
  process.exit(code === null ? 1 : code);
});

child.on("error", (err) => {
  console.error("Error executing cottage binary:", err);
  process.exit(1);
});

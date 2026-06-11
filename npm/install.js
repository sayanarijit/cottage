const fs = require("fs");
const path = require("path");
const https = require("https");
const crypto = require("crypto");
const { spawnSync } = require("child_process");

const pkg = require("./package.json");
const version = pkg.version;
const platform = process.platform;
const arch = process.arch;

let target = "";
if (platform === "darwin") {
  if (arch === "arm64") {
    target = "aarch64-apple-darwin";
  } else {
    console.error(`Unsupported macOS architecture: ${arch}`);
    process.exit(1);
  }
} else if (platform === "linux") {
  if (arch === "x64") {
    target = "x86_64-unknown-linux-musl";
  } else if (arch === "arm64") {
    target = "aarch64-unknown-linux-musl";
  } else if (arch === "ia32") {
    target = "i686-unknown-linux-musl";
  } else {
    console.error(`Unsupported Linux architecture: ${arch}`);
    process.exit(1);
  }
} else if (platform === "win32") {
  if (arch === "x64") {
    target = "x86_64-pc-windows-msvc";
  } else if (arch === "arm64") {
    target = "aarch64-pc-windows-msvc";
  } else {
    console.error(`Unsupported Windows architecture: ${arch}`);
    process.exit(1);
  }
} else if (platform === "freebsd") {
  if (arch === "x64") {
    target = "x86_64-unknown-freebsd";
  } else {
    console.error(`Unsupported FreeBSD architecture: ${arch}`);
    process.exit(1);
  }
} else {
  console.error(`Unsupported platform: ${platform}`);
  process.exit(1);
}

const url = `https://github.com/sayanarijit/cottage/releases/download/v${version}/cottage-${target}-v${version}.tar.gz`;
const binDir = path.join(__dirname, "bin");

if (!fs.existsSync(binDir)) {
  fs.mkdirSync(binDir, { recursive: true });
}

console.log(`Downloading cottage binary for ${target} from ${url}...`);

function download(url, dest, callback, redirectCount = 0) {
  if (redirectCount > 5) {
    callback(new Error("Too many redirects"));
    return;
  }
  https
    .get(url, (res) => {
      if (res.statusCode === 302 || res.statusCode === 301) {
        const nextUrl = res.headers.location;
        if (!nextUrl) {
          callback(new Error("Redirect location header missing"));
          return;
        }
        let absoluteUrl = nextUrl;
        if (nextUrl.startsWith("/")) {
          const parsed = new URL(url);
          absoluteUrl = `${parsed.protocol}//${parsed.host}${nextUrl}`;
        }
        if (!absoluteUrl.startsWith("https://")) {
          callback(new Error(`Insecure redirect URL: ${absoluteUrl}`));
          return;
        }
        download(absoluteUrl, dest, callback, redirectCount + 1);
        return;
      }
      if (res.statusCode !== 200) {
        callback(
          new Error(`Failed to download binary: HTTP ${res.statusCode}`)
        );
        return;
      }
      const file = fs.createWriteStream(dest);
      res.pipe(file);
      file.on("finish", () => {
        file.close(callback);
      });
    })
    .on("error", (err) => {
      fs.unlink(dest, () => {});
      callback(err);
    });
}

function downloadToString(url, callback, redirectCount = 0) {
  if (redirectCount > 5) {
    callback(new Error("Too many redirects"));
    return;
  }
  https
    .get(url, (res) => {
      if (res.statusCode === 302 || res.statusCode === 301) {
        const nextUrl = res.headers.location;
        if (!nextUrl) {
          callback(new Error("Redirect location header missing"));
          return;
        }
        let absoluteUrl = nextUrl;
        if (nextUrl.startsWith("/")) {
          const parsed = new URL(url);
          absoluteUrl = `${parsed.protocol}//${parsed.host}${nextUrl}`;
        }
        if (!absoluteUrl.startsWith("https://")) {
          callback(new Error(`Insecure redirect URL: ${absoluteUrl}`));
          return;
        }
        downloadToString(absoluteUrl, callback, redirectCount + 1);
        return;
      }
      if (res.statusCode !== 200) {
        callback(
          new Error(`Failed to download checksum: HTTP ${res.statusCode}`)
        );
        return;
      }
      let data = "";
      res.on("data", (chunk) => {
        data += chunk;
      });
      res.on("end", () => {
        callback(null, data);
      });
    })
    .on("error", (err) => {
      callback(err);
    });
}

function verifyChecksum(filePath, expectedSha256, callback) {
  const hash = crypto.createHash("sha256");
  const stream = fs.createReadStream(filePath);

  stream.on("data", (data) => {
    hash.update(data);
  });

  stream.on("end", () => {
    const computedSha256 = hash.digest("hex");
    if (computedSha256 === expectedSha256) {
      callback(null);
    } else {
      callback(
        new Error(
          `Checksum mismatch! Expected: ${expectedSha256}, Got: ${computedSha256}`
        )
      );
    }
  });

  stream.on("error", (err) => {
    callback(err);
  });
}

const tempFile = path.join(__dirname, "cottage.tar.gz");
const checksumUrl = `https://github.com/sayanarijit/cottage/releases/download/v${version}/cottage-${target}-v${version}.sha256`;

console.log(`Downloading checksum from ${checksumUrl}...`);
downloadToString(checksumUrl, (err, checksumContent) => {
  if (err) {
    console.error("Error downloading checksum:", err.message);
    process.exit(1);
  }

  const match = checksumContent.trim().match(/^([a-f0-9]{64})/i);
  if (!match) {
    console.error("Invalid checksum file format.");
    process.exit(1);
  }
  const expectedSha256 = match[1].toLowerCase();

  console.log(`Downloading cottage binary for ${target} from ${url}...`);
  download(url, tempFile, (err) => {
    if (err) {
      console.error("Error downloading binary:", err.message);
      process.exit(1);
    }

    console.log("Verifying binary checksum...");
    verifyChecksum(tempFile, expectedSha256, (err) => {
      if (err) {
        console.error("Checksum verification failed:", err.message);
        try {
          fs.unlinkSync(tempFile);
        } catch (e) {}
        process.exit(1);
      }

      console.log("Extracting binary using system tar...");
      const tarResult = spawnSync("tar", ["-xzf", tempFile, "-C", binDir], {
        stdio: "inherit",
      });

      // Clean up the temp file
      try {
        fs.unlinkSync(tempFile);
      } catch (e) {}

      if (tarResult.status !== 0) {
        console.error("Failed to extract binary.");
        process.exit(1);
      }

      // Ensure binaries are executable on Unix/macOS
      if (platform !== "win32") {
        const ctgPath = path.join(binDir, "ctg");
        const ctgxPath = path.join(binDir, "ctgx");
        try {
          if (fs.existsSync(ctgPath)) fs.chmodSync(ctgPath, 0o755);
          if (fs.existsSync(ctgxPath)) fs.chmodSync(ctgxPath, 0o755);
        } catch (e) {
          console.warn(
            "Warning: Failed to set executable permissions on binaries:",
            e.message
          );
        }
      }

      console.log("Cottage binary installed successfully.");
      process.exit(0);
    });
  });
});

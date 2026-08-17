#!/usr/bin/env bash
# Build the bilingual HAgency book into a single deployable site:
#   dist/
#   ├── index.html   (language auto-detect redirect)
#   ├── zh/          (Chinese book)
#   └── en/          (English book)
# Suitable for GitHub Pages (serve the dist/ directory).
set -euo pipefail
cd "$(dirname "$0")"

rm -rf dist
mkdir -p dist

(cd zh && mdbook build)
(cd en && mdbook build)
cp -r zh/book dist/zh
cp -r en/book dist/en

cat > dist/index.html <<'HTML'
<!DOCTYPE html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8">
  <title>HAgency</title>
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <script>
    // Redirect to the reader's preferred language; default to Chinese.
    var lang = (navigator.language || "zh").toLowerCase();
    var target = lang.indexOf("zh") === 0 ? "zh/" : "en/";
    window.location.replace(target);
  </script>
</head>
<body>
  <noscript>
    <p><a href="zh/">中文</a> | <a href="en/">English</a></p>
  </noscript>
</body>
</html>
HTML

echo "Built bilingual site at $(pwd)/dist"

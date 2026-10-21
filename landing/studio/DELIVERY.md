# Website asset delivery

Edit `home.css`, `features.css`, `how-it-works.css`, and `readability.css` as the stylesheet sources. Then run:

```sh
node scripts/build-studio-delivery.mjs
```

This rebuilds the marked, inline style block in each HTML page in cascade order. The small page-specific style payload ships with the document to avoid render-blocking stylesheet requests, including with JavaScript disabled. `npm run build:landing` runs this step automatically during deployment. Do not hand-edit the generated style blocks.

The website uses `brand/studio-logo-320.webp` and `brand/studio-logo-640.webp` with responsive image sizes. The high-resolution PNG remains the editable master and is not requested by the page. To regenerate the WebP files, install Pillow and run:

```sh
python scripts/prepare-studio-web-logo.py
```

Generated logo assets are committed, so deployment does not need Python or Pillow. The optimizer crops transparent padding, preserves transparency, and produces the normal and high-density variants. Keep the image dimensions and responsive `sizes` attributes aligned with the logo's CSS layout boxes.

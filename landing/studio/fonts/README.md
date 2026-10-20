Geist and Geist Mono are distributed under the SIL Open Font License, included
alongside the fonts. These variable WOFF2 Latin subsets serve the English Studio
pages; other characters use the system fallback. No runtime font service is used.

Sources (Google Fonts):

- Geist v5: https://fonts.gstatic.com/s/geist/v5/gyByhwUxId8gMEwcGFU.woff2
- Geist Mono v6: https://fonts.gstatic.com/s/geistmono/v6/or3nQ6H-1_WfwkMZI_qYFrcdmg.woff2
- Licenses: https://github.com/google/fonts/tree/main/ofl/geist and https://github.com/google/fonts/tree/main/ofl/geistmono

Versioned filenames allow long-lived caching. Use a new filename when replacing
a font, and update the CSS. Fonts are deliberately not preloaded: on a slow
connection, critical HTML and CSS take priority and optional display uses the
system fallback for that visit without a late layout shift.

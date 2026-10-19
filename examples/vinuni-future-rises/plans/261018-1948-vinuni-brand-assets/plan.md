---
status: done
created: 2026-10-18
follows: ../../../../plans/261018-1900-vinuni-connected-introduction/plan.md
---

# VinUni brand and site assets in the film

## Outcome

Wherever the film names a college or a student activity it also shows the `VinUniversity`
logo and photos from vinuni.edu.vn, and the whole film uses the brand's colours and type.

## Constraints and non-goals

- Footage keeps filling the screen; logos, lockups and photo cards are overlays, the reels
  and the 2 s scene grid are unchanged.
- Evergreen material only: no photos with year banners, news thumbnails, named people,
  stipends or partner logos.
- Site assets are fetched by `tools/fetch-media.py` into git-ignored `dynamic_media/`.
- Not a new storyline, no new scenes.

## Brand (sampled, not invented)

| Token | Value | Source |
| --- | --- | --- |
| `NAVY` | `#002147` | site CSS `--home-text-dark`; background, washes |
| `BLUE` | `#1C4078` | logo chevron; lockup rule, globe |
| `AMBER` | `#F9A730` | site CSS `--home-orange`; the accent: rules, stars, E·X·C·E·L, globe, chapter bar |
| `CECS` | `#C72127` | cecs.vinuni.edu.vn CSS `--primary` (red) |
| `CHS` | `#018369` | chs.vinuni.edu.vn CSS `--primary` (green) |
| `CBM` | `#FFA200` | cbm.vinuni.edu.vn CSS `--primary` (yellow) |
| `CAS` | `#75A7DC` | cas.vinuni.edu.vn CSS `--primary` (blue) |
| `SKY` | `#E0ECF9` | site CSS `--home-blue-light`; labels, body, grade highlights |
| Display | Montserrat 700 | site headings (Google Fonts link on every page) |
| Labels | Montserrat 600 | spaced capitals, replaces JetBrains Mono |
| Body | Mulish 400 | site body font |

Fonts come from their source repositories (googlefonts/mulish, JulietaUla/Montserrat);
the OFL texts are in `licenses/`. Gold, the old navy `#0A1128` and JetBrains Mono are gone
from code and shaders (grade duotone, particles, aurora).

## Logo

VinUniversity serves two official horizontal logos: vinuni.edu.vn's header uses
`logo-vinuni.png` (flat chevron, blue "UNIVERSITY", 1200x229), the four college sites use
`header-logo.png` (faceted chevron, black "UNIVERSITY", 711x153). The film uses the
college sites' version, since it stands next to the college names and matches their
faceted marks; switching back is the `logo.png` URL in `fetch-media.py` and
`LOGO_RATIO` in `art.rs`. No site publishes a reversed logo, so the recoloured
`logo-white.png` an earlier pass derived is gone and the logo always sits on white: a
plate in the opening and the clubs scene, the card strips, the college lockups.

The college sites publish no separate logo files (header and footer logos are
byte-identical on all four). Each college's identity is its `--primary` colour and a
faceted triangle in that colour on its home page (`Group-243/79/242/241.png`, fetched as
`mark-cecs/chs/cbm/cas.png`). Each college scene uses both: the lockup's edge in the
college colour with the mark in its bottom right corner, the kicker rule and the photo
captions' bars in that colour. On the four-college overview each card carries its
college's mark and colour bar, and the kicker rule is neutral light blue so CBM's yellow
is not confused with the global amber.

## Facts used (college sites, student activities page)

- Names: College of Engineering & Computer Science, Health Sciences, Business &
  Management, Arts & Sciences (page titles and hero text).
- Lines: "Shape the future of innovation" (CECS), "Health leaders to advance the future of
  care" (CHS), "developing future entrepreneurs … leaders" (CBM), "Redefining personalized
  learning" (CAS).
- Captions: interdisciplinary projects, innovation and research (CECS); VinUniversity
  Bachelor of Nursing, 3D Technology in Medicine Center (CHS; each photo captioned with the
  heading it sits under); "all curriculum
  and academic programs validated by Cornell University" (CBM); social sciences and
  interdisciplinary humanities, Economics and Psychology programs (CAS).
- Events: Week of Welcome (begins after the two-day Move-in Day), Club Fair ("over 30 clubs"), Green
  Sports Day (tug of war among its team activities), Spring Festival & Musical Show.

## Changes

| Scene | Added |
| --- | --- |
| Opening | the logo on a white plate wipes in instead of typed VINUNIVERSITY |
| Colleges | four cards with the academics page's college photos, each college's mark and colour, the logo, names under them |
| Engineering … Arts | lockup in the college's colour with its mark, the college's own line, two photo cards from its site |
| Campus | Week of Welcome move-in photo card with the logo |
| Clubs | the three named events with their photos, the logo on a plate over the kicker |
| Excellence | stars in the site's amber (red read as a warning on stars and big letters) |
| Rise | credit names photos and the college sites |

New photos (`fetch-media.py` `CARDS`, fitted to 1600 px): `card-*` (academics page),
`engineering-*`, `health-*`, `business-*`, `arts-*` (college sites), `life-*` (student
activities page). Rejected: a Gala Dinner photo with a cohort banner, a CECS poster naming
a student and a stipend, CBM partner logos, a CAS delegation visit. The Green Sports Day
photo prints its date on a banner in the top right corner, which `fetch-media.py` crops
off.

## Result

`vinuni-future-rises.mp4`: 7200 frames, 120.0 s, 1080p60 H.264 + AAC, -14.2 LUFS, -1.7
dBTP, no clipped samples. `inspect --every 0.25s`: 497 frames, no problems (fonts, glyphs
and the logo and mark images resolve). Clippy (`-D warnings`), fmt and 5 tests pass; no
year appears in on-screen strings. Frames checked: the four college lockups in red, green,
yellow and blue with their marks, the overview cards with marks and colour bars, the logo
plates in the opening and the clubs scene.

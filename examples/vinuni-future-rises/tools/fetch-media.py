"""Downloads the VinUniversity footage, photos and logo this film is cut from into
dynamic_media/ (git-ignored: the assets are (c) VinUniversity and are not committed).

Sources are the vinuni.edu.vn homepage banner and photos, photos from its academics and
student activities pages and the four college sites (cecs/chs/cbm/cas.vinuni.edu.vn), and
videos of VinUniversity's official YouTube channel (downloaded with the standalone yt-dlp.exe: pass --yt-dlp PATH
or put it on PATH). Every clip is cut inside one scene of its source (stroll, labclass,
posters and tennis keep a cut of the source edit), cropped so the burned-in subtitles (and,
for some videos, the channel logo) are outside the frame, slowed to cover its slot (at most
1.5x for 24 fps sources, 1.6x for the slow graduate silhouette), brought to 60 fps (motion
interpolation, or repeated frames for REPEAT_FRAMES) and re-encoded with a short GOP so the
parallel render workers seek cheaply. Needs ffmpeg on PATH.
Run: py tools/fetch-media.py [--yt-dlp C:/path/to/yt-dlp.exe]
"""
import argparse
import concurrent.futures
import os
import pathlib
import shutil
import subprocess
import sys
import time
import urllib.error
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parents[1]
OUT = ROOT / 'dynamic_media'
CACHE = ROOT / '.cache'
SITE = 'https://vinuni.edu.vn/wp-content/uploads/'
UA = 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0 Safari/537.36'

BANNER = '2025/04/Banner-Homepage.mp4'
CECS = 'https://cecs.vinuni.edu.vn/wp-content/uploads/sites/5/'
CHS = 'https://chs.vinuni.edu.vn/wp-content/uploads/sites/8/'
CBM = 'https://cbm.vinuni.edu.vn/wp-content/uploads/sites/7/'
CAS = 'https://cas.vinuni.edu.vn/wp-content/uploads/sites/4/'
PHOTOS = {
    # the header logo of the college sites (faceted chevron, black wordmark)
    'logo.png': 'https://cecs.vinuni.edu.vn/wp-content/themes/vinuni2023/vinuni-modules/home/assets/images/header-logo.png',
    # each college's mark: the faceted triangle in its colour from its home page
    'mark-cecs.png': CECS + '2024/03/Group-243.png',
    'mark-chs.png': CHS + '2024/03/Group-79.png',
    'mark-cbm.png': CBM + '2024/03/Group-242.png',
    'mark-cas.png': CAS + '2024/03/Group-241.png',
    'exchange.jpg': '2026/10/Studying-at-VinUniversity-4.jpg',
    'graduation.jpg': '2026/10/Studying-at-VinUniversity-1.jpg',
    'steps.jpg': '2026/10/Studying-at-VinUniversity-2.jpg',
}
# photos shown on cards next to the college and activity names, from the academics page,
# the four college sites and the student activities page; fitted to 1600 px wide
CARDS = {
    'card-engineering.jpg': SITE + '2024/03/CS-1-2-e1711804823355.jpg',
    'card-health.jpg': SITE + '2024/03/CHS-opt-1-1-2-e1713890889721.jpg',
    'card-business.jpg': SITE + '2024/03/CBM-opt-1-1-2-e1711804794732.jpg',
    'card-arts.jpg': SITE + '2024/03/CAS-opt-1-2-2-e1711804664810.jpg',
    'engineering-team.jpg': CECS + '2024/03/TIE03005-scaled-1.jpg',
    'engineering-robotics.jpg': CECS + '2024/03/compressed_DSCF0693-1.jpg',
    'health-nursing.jpg': CHS + '2024/03/DSCF0426-1.png',
    'health-3d.jpg': CHS + '2024/03/DSCF0559-11.png',
    'business-team.jpg': CBM + '2024/03/DSCF1090-min-1.jpg',
    'business-study.jpg': CBM + '2024/03/DSF3696-min.jpg',
    'arts-campus.jpg': CAS + '2024/03/banner-cuoi-GE.jpg',
    'arts-study.jpg': CAS + '2024/03/strategic-collab-e1711360771117.jpg',
    'life-move-in.jpg': SITE + '2024/03/HNE-05909.jpg',
    'life-sports-day.jpg': SITE + '2024/03/HNE-00684.jpg',
    'life-club-fair.jpg': SITE + '2024/03/HNE-08597.jpg',
    'life-spring-festival.jpg': SITE + '2024/03/Student-Activity-1.jpg',
}
# crops before fitting: the sports day banner in the top right corner prints its date
CARD_CROPS = {'life-sports-day.jpg': 'crop=iw*0.78:ih:0:0,'}
# official VinUniversity channel: anniversary, "We are the Future Makers",
# "One Day at VinUniversity", Ignite Week, "Why VinUni?" (international students),
# the graduation films "Legacy of Excellence" and "Soaring", the Career Day film, "Where
# The Future Rises", "Welcome Cohort 6", Green Sports Day and the four college films
YOUTUBE = ['HEj956e60Ng', 'KUGbRcUlXr0', 'D039cYmeUDA', 'pvPU8vaRnNU', '5Io91OsHbIE', 'dPWZDM5HujQ',
           'afS5OOQWReA', 'BvOySZfXag8', '1fFl-RMs_3Q', 'ug5271nzvSQ', '-sD3en0LTXY', 'g9UVW2EqQvk',
           'sdZEvi-JlkY', 'AqyffbZBgnI', 's86FANH94iA']
# each college's lockup (the VinUniversity logo over the college's name) from the white end
# card of the college's film: video, seconds before its end, height of the 1080p area
# searched from 20% down (CECS: its tagline sits under the lockup)
LOCKUPS = {
    'lockup-cecs.png': ('g9UVW2EqQvk', 4.0, 345),
    'lockup-cbm.png': ('sdZEvi-JlkY', 2.0, 594),
    'lockup-cas.png': ('AqyffbZBgnI', 1.5, 594),
    'lockup-chs.png': ('s86FANH94iA', 1.0, 594),
}
# employers and universities: 960 px renderings of their logos on Wikimedia Commons (public
# domain as text or simple shapes, still trademarks), NUS and NTU from English Wikipedia
# (non-free, used there under fair use)
COMMONS = 'https://thumb.wikimedia.org/wikipedia/commons/thumb/'
LOGOS = {
    'logo-google.png': COMMONS + '2/2f/Google_2015_logo.svg/960px-Google_2015_logo.svg.png',
    'logo-qualcomm.png': COMMONS + 'f/fc/Qualcomm-Logo.svg/960px-Qualcomm-Logo.svg.png',
    'logo-bcg.png': COMMONS + 'd/d0/Boston_Consulting_Group_2020_logo.svg/960px-Boston_Consulting_Group_2020_logo.svg.png',
    'logo-unilever.png': COMMONS + '8/87/Unilever_text_logo.svg/960px-Unilever_text_logo.svg.png',
    'logo-pg.png': COMMONS + '7/7a/P%26G_logo.svg/960px-P%26G_logo.svg.png',
    'logo-mckinsey.png': COMMONS + 'e/e2/McKinsey_and_Company_Logo_1.svg/960px-McKinsey_and_Company_Logo_1.svg.png',
    'logo-ibm.png': COMMONS + '5/51/IBM_logo.svg/960px-IBM_logo.svg.png',
    'logo-bosch.png': COMMONS + '1/16/Bosch-logo.svg/960px-Bosch-logo.svg.png',
    'logo-harvard.png': COMMONS + '7/70/Harvard_University_logo.svg/960px-Harvard_University_logo.svg.png',
    'logo-cornell.png': COMMONS + '4/4b/Cornell_University_logo.svg/960px-Cornell_University_logo.svg.png',
    'logo-penn.png': COMMONS + '9/92/University_of_Pennsylvania_wordmark.svg/960px-University_of_Pennsylvania_wordmark.svg.png',
    'logo-duke.png': COMMONS + 'e/e6/Duke_University_logo.svg/960px-Duke_University_logo.svg.png',
    'logo-epfl.png': COMMONS + '9/95/Logo_EPFL_2019.svg/960px-Logo_EPFL_2019.svg.png',
    'logo-nus.png': 'https://thumb.wikimedia.org/wikipedia/en/thumb/9/9b/NationalUniversityofSingapore.svg/960px-NationalUniversityofSingapore.svg.png',
    'logo-ntu.png': 'https://thumb.wikimedia.org/wikipedia/en/thumb/c/c6/Nanyang_Technological_University.svg/960px-Nanyang_Technological_University.svg.png',
}
# Wikimedia asks for a descriptive User-Agent and rate-limits bursts
WIKI_UA = 'vinuni-future-rises/1.0 (fframes example; render tool)'
CROPS = {
    'none': None,
    # subtitles sit at 86-96% of the height
    'sub': 'crop=iw*0.84:ih*0.84:iw*0.08:0',
    # also drops the channel's logo in the top-right corner and the higher subtitles of
    # pvPU8vaRnNU (82-95%)
    'logo': 'crop=iw*0.74:ih*0.74:iw*0.13:ih*0.04',
    # 16:9 inside a letterboxed shot (picture at 12-95% of the height), above its subtitles
    'letterbox': 'crop=iw*0.714:ih*0.714:iw*0.143:ih*0.126',
    # 16:9 inside the graduation film's picture band (10-90% of the height), above its
    # subtitles
    'band': 'crop=iw*0.79:ih*0.79:iw*0.105:ih*0.105',
    # the college films' subtitles start at 81% of the height
    'high': 'crop=iw*0.78:ih*0.78:iw*0.11:0',
    # the business film: picture at 10-90%, subtitles inside it from 72%
    'cbm': 'crop=iw*0.62:ih*0.62:iw*0.19:ih*0.1',
}
# clip: (source, shot in, shot out, crop, seconds it covers on screen)
# fast motion (hands, rackets, dancing) that motion interpolation smears into doubles:
# repeat frames instead
REPEAT_FRAMES = {'highfive.mp4', 'tennis.mp4', 'stage.mp4', 'hoops.mp4'}
CLIPS = {
    # who we are
    'aerial.mp4': ('HEj956e60Ng', 20.0, 24.4, 'sub', 6.5),
    'approach.mp4': ('KUGbRcUlXr0', 52.0, 53.5, 'letterbox', 2.25),
    'rooftops.mp4': ('HEj956e60Ng', 152.1, 156.5, 'sub', 6.65),
    'stroll.mp4': ('5Io91OsHbIE', 196.6, 201.7, 'logo', 5.1),
    'hall.mp4': ('5Io91OsHbIE', 86.5, 88.5, 'logo', 3.0),
    'cap.mp4': ('KUGbRcUlXr0', 124.6, 127.8, 'sub', 4.5),
    # what you study
    'lecture.mp4': ('D039cYmeUDA', 44.9, 46.6, 'logo', 2.5),
    'lecturewide.mp4': ('D039cYmeUDA', 46.8, 48.4, 'logo', 2.4),
    'chalk.mp4': ('D039cYmeUDA', 48.6, 52.5, 'logo', 4.25),
    'machine.mp4': ('HEj956e60Ng', 131.55, 132.7, 'sub', 1.7),
    'printer.mp4': ('pvPU8vaRnNU', 233.2, 235.3, 'logo', 3.15),
    'computers.mp4': ('banner', 12.35, 15.15, 'none', 4.2),
    'surgery.mp4': ('banner', 7.43, 9.82, 'none', 3.5),
    'sim.mp4': ('5Io91OsHbIE', 177.5, 179.7, 'logo', 3.2),
    'discuss.mp4': ('KUGbRcUlXr0', 214.05, 215.6, 'sub', 2.3),
    'meeting.mp4': ('5Io91OsHbIE', 221.4, 224.3, 'logo', 4.4),
    'lab.mp4': ('banner', 9.93, 12.23, 'none', 3.4),
    'labclass.mp4': ('5Io91OsHbIE', 72.6, 76.9, 'logo', 4.3),
    # from the colleges' own films
    'chshall.mp4': ('s86FANH94iA', 83.7, 86.6, 'high', 2.7),
    'hackteam.mp4': ('sdZEvi-JlkY', 45.3, 48.3, 'cbm', 2.7),
    'casclass.mp4': ('AqyffbZBgnI', 65.7, 67.25, 'high', 2.3),
    # how you learn
    'laptops.mp4': ('5Io91OsHbIE', 154.8, 156.6, 'logo', 3.0),
    'tables.mp4': ('pvPU8vaRnNU', 143.7, 145.8, 'logo', 2.8),
    'posters.mp4': ('pvPU8vaRnNU', 203.6, 207.0, 'logo', 3.4),
    'pitch.mp4': ('g9UVW2EqQvk', 18.3, 20.75, 'sub', 2.7),
    # the world
    'friends.mp4': ('5Io91OsHbIE', 250.1, 251.8, 'logo', 3.4),
    'chat.mp4': ('KUGbRcUlXr0', 102.7, 104.0, 'sub', 1.9),
    # life on campus
    'corridor.mp4': ('5Io91OsHbIE', 190.3, 192.7, 'logo', 3.0),
    'canteen.mp4': ('D039cYmeUDA', 52.8, 55.6, 'logo', 2.8),
    'walkers.mp4': ('5Io91OsHbIE', 193.5, 196.3, 'logo', 3.2),
    'tennis.mp4': ('D039cYmeUDA', 87.6, 90.2, 'logo', 2.6),
    'champions.mp4': ('HEj956e60Ng', 135.05, 136.15, 'sub', 1.65),
    'stage.mp4': ('5Io91OsHbIE', 6.9, 8.5, 'logo', 2.6),
    'highfive.mp4': ('D039cYmeUDA', 80.9, 84.05, 'logo', 4.7),
    # your future: where graduates go
    'gowns.mp4': ('dPWZDM5HujQ', 300.98, 302.45, 'band', 2.4),
    'careerday.mp4': ('afS5OOQWReA', 139.5, 142.25, 'none', 3.0),
    'hallgrads.mp4': ('dPWZDM5HujQ', 304.5, 307.2, 'band', 3.45),
    'burst.mp4': ('KUGbRcUlXr0', 188.8, 190.4, 'sub', 2.4),
    'hand.mp4': ('KUGbRcUlXr0', 193.6, 195.7, 'sub', 3.0),
    # a slow silhouette: holds up to 1.6x
    'graduate.mp4': ('KUGbRcUlXr0', 180.2, 182.35, 'sub', 3.45),
    'tower.mp4': ('KUGbRcUlXr0', 234.75, 236.15, 'sub', 2.1),
    # more of life on campus and your future
    'hoops.mp4': ('ug5271nzvSQ', 48.95, 50.38, 'sub', 2.1),
    'pool.mp4': ('-sD3en0LTXY', 45.95, 49.55, 'sub', 3.6),
    'booths.mp4': ('afS5OOQWReA', 110.55, 112.92, 'none', 3.7),
    'sashes.mp4': ('BvOySZfXag8', 252.4, 253.85, 'sub', 1.7),
    'flags.mp4': ('BvOySZfXag8', 253.95, 255.85, 'sub', 1.9),
    'spire.mp4': ('ug5271nzvSQ', 65.0, 66.6, 'sub', 1.85),
    # the intro montage, one beat each
    'sunrise.mp4': ('BvOySZfXag8', 49.05, 52.85, 'sub', 4.6),
    'machineshop.mp4': ('g9UVW2EqQvk', 131.45, 133.82, 'sub', 2.45),
    'labteach.mp4': ('g9UVW2EqQvk', 133.95, 135.38, 'sub', 2.4),
    'hackathon.mp4': ('ug5271nzvSQ', 47.0, 48.85, 'sub', 2.3),
    'studypair.mp4': ('g9UVW2EqQvk', 199.25, 203.8, 'sub', 2.6),
    'arrivals.mp4': ('ug5271nzvSQ', 43.0, 46.85, 'sub', 4.6),
    'aerialgold.mp4': ('BvOySZfXag8', 86.0, 89.85, 'sub', 4.6),
    'horizon.mp4': ('1fFl-RMs_3Q', 109.5, 113.95, 'sub', 4.7),
}


def download(path, target):
    if target.exists():
        return
    url = path if path.startswith('https://') else SITE + path
    request = urllib.request.Request(url, headers={'User-Agent': UA})
    with urllib.request.urlopen(request) as response:
        target.write_bytes(response.read())
    print('downloaded', target.name)


def card(ffmpeg, name, url):
    target = OUT / name
    if target.exists():
        return
    source = CACHE / 'photos' / url.rsplit('/', 1)[1]
    download(url, source)
    subprocess.run([ffmpeg, '-v', 'error', '-y', '-i', str(source), '-vf', CARD_CROPS.get(name, '') + "scale='min(1600,iw)':-2",
                    '-q:v', '3', str(target)], check=True)


def wiki_logo(name, url):
    target = OUT / name
    if target.exists():
        return
    request = urllib.request.Request(url, headers={'User-Agent': WIKI_UA})
    for attempt in range(4):
        try:
            with urllib.request.urlopen(request) as response:
                target.write_bytes(response.read())
            print('downloaded', name)
            break
        except urllib.error.HTTPError as error:
            if error.code != 429 or attempt == 3:
                raise
            time.sleep(10 * (attempt + 1))
    time.sleep(3)


def college_lockup(ffmpeg, name, video_id, before_end, h):
    """The lockup on the white end card of a college's film, its background keyed out (the
    lockup is dark and saturated, the card near white) and cropped to the lockup."""
    target = OUT / name
    if target.exists():
        return
    path = CACHE / 'yt' / f'{video_id}.mp4'
    duration = float(subprocess.run(['ffprobe', '-v', 'error', '-show_entries', 'format=duration', '-of', 'csv=p=0',
                                     str(path)], capture_output=True, text=True, check=True).stdout)
    # the middle of the card: the subtitles sit below it
    w = 1344
    raw = subprocess.run([ffmpeg, '-v', 'error', '-ss', f'{duration - before_end:.3f}', '-i', str(path), '-frames:v', '1',
                          '-vf', f'scale=1920:1080,crop={w}:{h}:288:216', '-f', 'rawvideo', '-pix_fmt', 'rgba', '-'],
                         capture_output=True, check=True).stdout
    pixels = bytearray(raw)
    xs, ys = [], []
    for y in range(h):
        for x in range(w):
            i = (y * w + x) * 4
            alpha = max(0, min(255, (215 - min(pixels[i:i + 3])) * 255 // 95))
            pixels[i + 3] = alpha
            if alpha > 64:
                xs.append(x)
                ys.append(y)
    x0, x1, y0, y1 = max(0, min(xs) - 6), min(w, max(xs) + 7), max(0, min(ys) - 6), min(h, max(ys) + 7)
    subprocess.run([ffmpeg, '-v', 'error', '-y', '-f', 'rawvideo', '-pix_fmt', 'rgba', '-s', f'{w}x{h}', '-i', '-',
                    '-vf', f'crop={x1 - x0}:{y1 - y0}:{x0}:{y0}', str(target)], input=bytes(pixels), check=True)
    print(f'lockup {name}: {x1 - x0}x{y1 - y0}')


def rgba(ffmpeg, path):
    """A PNG as (width, height, RGBA bytes)."""
    size = subprocess.run(['ffprobe', '-v', 'error', '-select_streams', 'v:0', '-show_entries', 'stream=width,height',
                           '-of', 'csv=p=0', str(path)], capture_output=True, text=True, check=True).stdout
    w, h = (int(v) for v in size.strip().split(','))
    raw = subprocess.run([ffmpeg, '-v', 'error', '-i', str(path), '-f', 'rawvideo', '-pix_fmt', 'rgba', '-'],
                         capture_output=True, check=True).stdout
    return w, h, bytearray(raw)


def name_line(w, h, pixels):
    """The box (x0, y0, x1, y1) of a lockup's bottom line, "College of ...". In the wide
    lockups only the part right of the rule (the last column inked over 80% of the height in
    the first 40% of the width) is searched."""
    left = 0
    if w / h > 3:
        rule = max(x for x in range(int(w * 0.4))
                   if sum(pixels[(y * w + x) * 4 + 3] > 64 for y in range(h)) > 0.8 * h)
        left = rule + 3
    ink = [any(pixels[(y * w + x) * 4 + 3] > 64 for x in range(left, w)) for y in range(h)]
    y1 = max(y for y in range(h) if ink[y]) + 1
    y0 = y1
    # up through the line, across gaps of a few rows (the dots of i), to the blank band above it
    while y0 > 0 and any(ink[max(0, y0 - 4):y0]):
        y0 -= 1
    xs = [x for y in range(y0, y1) for x in range(left, w) if pixels[(y * w + x) * 4 + 3] > 64]
    return min(xs), y0, max(xs) + 1, y1


def wide_lockups(ffmpeg):
    """The four colleges' lockups in one layout and one size: the arts lockup (mark, rule,
    VINUNIVERSITY, name line) with each college's own name line from its official lockup in
    place of the arts one, scaled to the same height, on canvases as wide as the longest."""
    targets = {name: OUT / name.replace('.png', '-wide.png') for name in LOCKUPS}
    if all(t.exists() for t in targets.values()):
        return
    w, h, template = rgba(ffmpeg, OUT / 'lockup-cas.png')
    tx0, ty0, tx1, ty1 = name_line(w, h, template)
    for y in range(ty0, ty1):
        for x in range(tx0, w):
            template[(y * w + x) * 4 + 3] = 0
    lines = {}
    for name in LOCKUPS:
        sw, sh, pixels = rgba(ffmpeg, OUT / name)
        x0, y0, x1, y1 = name_line(sw, sh, pixels)
        lines[name] = (x0, y0, x1, y1, round((x1 - x0) * (ty1 - ty0) / (y1 - y0)))
    width = max(w, max(tx0 + line[4] for line in lines.values()) + 6)
    blank = CACHE / 'lockup-template.png'
    subprocess.run([ffmpeg, '-v', 'error', '-y', '-f', 'rawvideo', '-pix_fmt', 'rgba', '-s', f'{w}x{h}', '-i', '-',
                    '-vf', f'pad={width}:{h}:0:0:color=0x00000000', str(blank)], input=bytes(template), check=True)
    for name, (x0, y0, x1, y1, line_w) in lines.items():
        subprocess.run([ffmpeg, '-v', 'error', '-y', '-i', str(blank), '-i', str(OUT / name), '-filter_complex',
                        f'[1]crop={x1 - x0}:{y1 - y0}:{x0}:{y0},scale={line_w}:{ty1 - ty0}:flags=lanczos[line];'
                        f'[0][line]overlay={tx0}:{ty0}:format=rgb,format=rgba', '-frames:v', '1',
                        str(targets[name])], check=True)
        print(f'lockup {targets[name].name}: {width}x{h}')


def fetch_youtube(yt_dlp, video_id):
    target = CACHE / 'yt' / f'{video_id}.mp4'
    if target.exists():
        return
    for _ in range(2):  # YouTube sometimes answers a range request with 403; retry once
        subprocess.run([yt_dlp, '--no-warnings', '-q', '-f', 'bv*[height<=1080][ext=mp4]/bv*[height<=1080]',
                        '-o', str(CACHE / 'yt' / '%(id)s.%(ext)s'), video_id], check=False)
        if target.exists():
            print('downloaded', target.name)
            return
    sys.exit(f'could not download {video_id}')


def cut(ffmpeg, name, spec):
    source, start, end, crop, cover = spec
    target = OUT / name
    if target.exists():
        return
    path = CACHE / 'banner-source.mp4' if source == 'banner' else CACHE / 'yt' / f'{source}.mp4'
    stretch = max(1.0, cover / (end - start))
    filters = [f for f in [CROPS[crop], 'scale=1920:1080:flags=lanczos',
                           f'setpts={stretch:.4f}*(PTS-STARTPTS)',
                           'fps=60' if name in REPEAT_FRAMES else
                           'minterpolate=fps=60:mi_mode=mci:mc_mode=aobmc:me_mode=bidir:vsbmc=1'] if f]
    subprocess.run([
        ffmpeg, '-v', 'error', '-y', '-ss', f'{start:.3f}', '-to', f'{end:.3f}', '-i', str(path),
        '-vf', ','.join(filters), '-an', '-c:v', 'libx264', '-preset', 'medium', '-crf', '17',
        '-g', '12', '-bf', '0', '-pix_fmt', 'yuv420p', str(target),
    ], check=True)
    print(f'clip {name}: {end - start:.2f}s x{stretch:.2f}')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--yt-dlp', default=shutil.which('yt-dlp') or shutil.which('yt-dlp.exe'))
    args = parser.parse_args()
    ffmpeg = shutil.which('ffmpeg')
    if not ffmpeg:
        sys.exit('ffmpeg is not on PATH')
    OUT.mkdir(exist_ok=True)
    (CACHE / 'yt').mkdir(parents=True, exist_ok=True)
    (CACHE / 'photos').mkdir(parents=True, exist_ok=True)
    for name, path in PHOTOS.items():
        download(path, OUT / name)
    for name, url in CARDS.items():
        card(ffmpeg, name, url)
    download(BANNER, CACHE / 'banner-source.mp4')
    missing = [v for v in YOUTUBE if not (CACHE / 'yt' / f'{v}.mp4').exists()]
    if missing and not args.yt_dlp:
        sys.exit('yt-dlp not found: pass --yt-dlp C:/path/to/yt-dlp.exe')
    for video_id in missing:
        fetch_youtube(args.yt_dlp, video_id)
    for name, url in LOGOS.items():
        wiki_logo(name, url)
    for name, (video_id, before_end, height) in LOCKUPS.items():
        college_lockup(ffmpeg, name, video_id, before_end, height)
    wide_lockups(ffmpeg)
    # motion interpolation is single-threaded per clip: cut the clips side by side
    with concurrent.futures.ThreadPoolExecutor(max(1, (os.cpu_count() or 2) // 2)) as pool:
        for job in [pool.submit(cut, ffmpeg, name, spec) for name, spec in CLIPS.items()]:
            job.result()


if __name__ == '__main__':
    main()

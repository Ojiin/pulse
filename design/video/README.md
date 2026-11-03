# The Pulse film

A product film of about a minute (1920 × 1080, 60 fps, no sound). All of its
content is made up (`data.js`); the panel in it is the real one from
`desktop/ui`, run in a browser with a stand-in for the Rust side (`mock.js`).

To render it again, run `node design/video/render.mjs` from the repo. It needs
Node 20 or later, Google Chrome, ffmpeg and the panel's packages
(`npm install` in `desktop/ui`). The script starts the panel's vite with
`vite.config.js` and drives Chrome over the DevTools protocol. It calls
`window.__frame(t)` in `film.html` for every frame and pipes the screenshots
to ffmpeg, which writes `out/pulse.mp4` in about five minutes. Each frame is a
function of its time only, so a render always comes out the same. A part
renders with `--from 13 --to 23`, single frames with `--still 17.5 40`, and
`--scale 2` draws at 3840 × 2160 and scales down. While the dev server runs,
`http://localhost:5288/design/video/film.html?t=17.5` shows one frame in a
browser and `?play` plays the film. The scenes and their timings are in
`film.js`, the look in `film.css`; `out/` is not kept in git.

`design/video/shrink.sh` makes smaller files from `out/pulse.mp4`:
`pulse-1080p.mp4` (60 fps, about 3.5 MB), `pulse-1080p30.mp4` (about 3 MB) and
`pulse-720p.mp4` (about 2 MB). Each stays under GitHub's 10 MB for a video in
the README, which is uploaded through the web editor; the video is not
committed.

## Scenes and music cues

| Time | Scene | Cue for music |
| --- | --- | --- |
| 0:00 | Sessions and routines everywhere: “So many sessions. So many routines.” | soft pad fades in |
| 0:03.9 | “Which one needs you?” | a held note, a little tension |
| 0:06.0 | Everything flies into the menu bar item | rise into the first beat |
| 0:07.9 | The counts tick up, “What needs you. Right in the menu bar.” | first beat, a light pluck on each count (7.9 / 8.15 / 8.4 / 8.65 / 8.9 / 9.15) |
| 0:13.2 | The panel opens, its cards settle: “Oldest first. Nothing slips through.” | the groove starts |
| 0:17.4 | The red “2 days ago” is ringed | accent |
| 0:23.5 | A notification: “Needs you: Price watch” | chime-like accent |
| 0:25.4 | The click, the session opens: “One click. Straight to the session.” | short click, then a lift |
| 0:30.9 | Commands to check: “The dev server you forgot? Pulse didn’t.” | accent on “forgotten?” at 0:33.0 |
| 0:37.5 | Timeline: “Your whole day.” | tab click, a new phrase |
| 0:41.0 | History, a search types itself: “Every session.” | tab click, light ticks while typing (41.9–42.8) |
| 0:46.1 | The panel is dragged aside | the music thins out |
| 0:48.4 | It is made small: “Out of your way. Whenever you like.” | small drop |
| 0:52.0 | Light turns to dark | a swell across the wipe |
| 0:54.3 | The icon, “Pulse”, the repo | resolve, the last chord |
| 1:01.0 | Fade to black | let it ring out, end at 1:02 |

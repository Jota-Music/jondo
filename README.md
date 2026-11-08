# Jondo

A minimal, native reimplementation of [Jota](https://github.com/Jota-Music/jota) in Rust and GPUI.

Jondo keeps Jota's visual language: stone surfaces, a 672px content column, the same Lucide icons,
the vinyl disc, cover-derived accent colour, and the responsive compact player. It replaces the
webview with native rendering and a small streaming audio engine.

## What works

- **Spotify sign-in** with PKCE and the librespot desktop client id, exchanged at the accesspoint for
  the reusable credentials Jota stores; no developer app, client secret or OAuth refresh token.
- **Spotify library** through the same private metadata endpoints Jota uses: rootlist, playlists, tracks.
- **YouTube search** and **audio playback** through InnerTube. It asks anonymously first and
  retries signed only when YouTube asks for it, using the session the panel imports from a
  supported browser (Chromium family and Firefox family jars) or from pasted cookies.
- **Local playlists**: create, rename, delete and add tracks from search.
- **Rooms**: join a Jota relay room, share playback controls, save rooms, and configure the
  relay from Settings.
- **Backup sync** through the same `jota.backup.json` Jota writes: playlists, follows, saved
  playlists, shelf order and the relay, with tombstones, per-record timestamps and Jota's merge
  rules. A folder you already sync on desktop — picked with the system dialog or typed by hand.
  The settings panel also exports and imports the document by hand, and carries the
  YouTube/Spotify sessions encrypted under a shared pass.
- **Settings** with the account pair, the interface language (English and Spanish), Discord rich
  presence, the relay, backups, logs and the version.
- **Player**: seek, volume, shuffle, repeat, queue, dominant-colour accent, current-track highlight.
- **Responsive desktop UI**: full player at wide widths, compact bottom bar and a full-player sheet
  below 768px.
- **Low memory**: streams AAC with an eight-second playback buffer and fetches the stream in 512KB
  chunks as the decoder reaches them, so a track is never downloaded whole; bounded cover texture
  cache.

Spotify playback is deliberately not implemented. Spotify supplies metadata; audio always comes from
YouTube.

## Build

```bash
cargo run --release
```

Linux needs ALSA, Vulkan and the usual GPUI development packages:

```bash
# Debian/Ubuntu
sudo apt install libasound2-dev libxkbcommon-dev libwayland-dev libx11-dev \
  libfontconfig-dev libfreetype-dev libvulkan-dev
```

Rust 1.88 or newer (the code uses let chains).

For the edit/run loop:

```bash
cargo dev
```

That is an alias for `cargo watch --no-vcs-ignores -w src -w assets -w Cargo.toml -x run`, defined in
`.cargo/config.toml`, and needs `cargo install cargo-watch` once. The flags are not optional: this
crate lives inside a large git repo, and plain `cargo watch` spends minutes walking it (every
`node_modules` and `.pnpm-store`) to discover `.gitignore` files before it runs anything. Watch the
crate's own files explicitly and skip that discovery.

## Platforms

GPUI renders through Vulkan and ships no Android backend, so this build targets desktop Linux,
macOS and Windows. The previous egui Android activity is not part of the GPUI build.

## Migration status

The GPUI port covers the shell, header and search panel, shelf, playlist page, YouTube search,
player and queue, create/rename/delete sheets, settings, following, artist/album/channel/profile
pages, the YouTube audio editor and saved rooms.

Interactions match the original: drag-to-reorder (shelf, playlist, queue and rooms), the rotating
vinyl, 150ms hover easing on buttons and rows, sheet/search/route/toast/cover transitions,
multi-select with its action bar, artist tabs and always-on-top pinning through X11.

Every song list is virtualized (`uniform_list` with a persistent scroll handle and the 15px
scrollbar gutter); the shelf, the following page and the rooms page scroll their tiles whole, and
each page rebuilds its row set only when the list, the query or the order changes. The rooms shelf
shows each saved room's relay status: live member count, connecting spinner, offline power icon, the
password lock and the amber warning when the host left.
The YouTube editor previews the picked video with play/pause, stop and a progress bar before saving.

## Backup

The library travels in `jota.backup.json`, the document Jota autosaves: `playlists` (with the shared
`songs`/`albums`/`artists` maps), `follows`, `saved`, `order`, `settings` and `sessions`. Jondo
writes the same bytes Jota writes, so both apps can share one folder.

- Desktop reads the folder directly and settles with a `stat` while nothing changed.
- Android picks the document with SAF (a folder through the tree picker, a cloud file one document
  at a time). The provider's copy is staged as `filesDir/sync/incoming.json`, merged here, and
  written back from `filesDir/sync/outgoing.json` with a truncating stream.
- A provider that serves a cached copy is not written over: the pass marks itself stale, reads once
  more and writes the union. The write baseline only moves once the commit confirmed the bytes, so a
  failed write is retried instead of being believed.
- `sessions` (the encrypted YouTube cookies and Spotify credentials) is carried through untouched:
  Jondo does not stage them, so a document written by Jota keeps them.

## Data

Environment variables: `DISCORD_CLIENT_ID` overrides the Discord application used for rich
presence (Jota's by default), and `JONDO_REPO` (`owner/name`) enables the release check in About —
without it the panel only links to the repository.

State lives in `~/.config/jondo/state.json` with mode `0600`. Next to it, `playlists/` holds the
cached track lists (so entering a playlist already seen is instant), `covers/` the cover thumbnails
and `library.json` the last Spotify library, so the shelf paints before the network answers. The
backup document itself lives in the folder you choose. `state.json` contains local playlists, the queue,
volume, the Spotify session (the same account and credential blob Jota keeps), and the saved relay and
rooms. Delete it to reset the app.

## Release

- Android: `./android/build.sh --release` builds both ABIs and packages a signed APK. Set
  `ANDROID_KEYSTORE_FILE`, `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS` and
  `ANDROID_KEY_PASSWORD`, or add a gitignored `android/keystore.properties`. The release build
  refuses to run without them.
- Name the artifacts `jondo.apk` and `jondo-<arch>.AppImage`, and publish a `SHA256SUMS` next to
  them: the updater verifies the checksum before installing.
- Build with `JONDO_REPO=Jota-Music/jondo` so the release check points at this repository.
- CI: `.github/workflows/ci.yml` checks format, clippy, tests and the Android build. Pushing a
  `vX.Y.Z` tag runs `.github/workflows/release.yml` and publishes `jondo.apk`,
  `jondo-x86_64.AppImage` and `SHA256SUMS`.
- GitHub secrets for the Android release: `ANDROID_KEYSTORE_BASE64`,
  `ANDROID_KEYSTORE_PASSWORD`, `ANDROID_KEY_ALIAS` and `ANDROID_KEY_PASSWORD`.

## Layout

The tree is feature-first: each business concept owns its layers, so a slice can be read
top to bottom without jumping across the crate. The dependency direction is `views → app →
domain`: `domain` holds the types, `app` the use cases and `views` the outermost consumers.
`domain` never depends on gpui, tokio or an adapter.

```
src/
  music/                  the product domain
    catalog/              discovering outside the library
      app/                search, album/artist/user/channel loading, row caches
      views/              search results and the user/artist/album/channel pages
    library/              what the user keeps
      app/                shelf, follows, local playlists, row caches
      views/              shelf, playlist page, create/rename/confirm/picker sheets
    player/
      domain/             queue, shuffle and repeat
      app/                player state, play/pause/seek, audio output and resolution
      views/              disc, full player, compact bar and queue
    rooms/
      domain/             room messages, roles and clock projection
      app/                connection, relay websocket, dial, sync, events and panel
      views/              saved rooms, cards and join panel
    track/
      domain/             YouTube editor mode
      app/                YouTube editor and preview state
      views/              rows, lists, credits, selection, search and the YouTube editor
  settings/
    domain/               Spotify sign-in state
    app/                  sessions, backup/sync, update and presence
    views/                account, interface, backups, relay and about
  shared/                 shared kernel, adapters and UI kit
    browser/app/          browser cookie jars and secrets
    crypto/app/           session encryption and the login foreground hold
    i18n/
      domain/             Locale value object
      app/                English and Spanish copy and active locale
    kernel/app/           clock, duration formatting, loading state and lock helpers
    logs/app/             logging install and capture
    music/
      domain/             Song, Playlist, Album, Artist, Channel, Follow, media and search
      app/                cover cache, Discord presence, Spotify, YouTube and list tables
      views/              cover artwork, mosaics and localized labels
    platform/app/         desktop and Android bridges, heap trimming
    storage/
      domain/             local store, playlist, session and sync report types
      app/                state file, playlist cache and library cache
    ui/
      views/              generic components and design tokens
  shell/                  composition root
    domain/               routes, link targets, commands, gestures and shell state types
    app/                  use cases: Jondo, navigation, jobs, events, clock, setup and drag
    views/                chrome, layouts, header, menu, sheet, page and toolbar
```

## Credits

- UI, assets and interaction model derived from [Jota](https://github.com/Jota-Music/jota) (GPL-3.0).
- Icons from [Lucide](https://lucide.dev) (ISC), rendered from the exact SVG sources.
- Spotify metadata through Spotify's private spclient, using the subset Jota and go-librespot rely on.

## Licence

GPL-3.0-only. See `LICENSE`.

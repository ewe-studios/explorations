# Tiny Skies WebGL Game -- Spec

## Source

- **Location:** `/home/darkvoid/Boxxed/@formulas/src.rust/src.llamacpp/src.webgl/tinyskies/`
- **Language:** TypeScript (monorepo: client, server, shared)
- **Framework:** Three.js for WebGL rendering, Socket.IO for multiplayer, Prisma/PostgreSQL for persistence
- **Deployment:** Vercel (client), Railway (server)

## What This Project Is

Tiny Skies is a 3D browser-based flight adventure game built with Three.js. Players fly a biplane over procedurally-generated island terrain, collect rings, complete quests (package delivery, landmark selfies, paintball combat), race against other players, interact with NPCs, and progress through vehicle upgrades (biplane → magic carpet → void vehicle → boat). The game features real-time multiplayer via WebSocket state sync, day/night cycles, atmospheric VFX (aurora, god rays, meteor showers, contrails, fireflies), and a progression system with XP/levels/unlocks.

## Documentation Goal

A reader should understand:

1. How the Three.js scene is structured and rendered
2. How procedural terrain generation works (Simplex noise, octaves, presets)
3. How flight controls and physics work (pitch, yaw, roll, drift)
4. How the multiplayer networking system works (Socket.IO, state interpolation)
5. How quests and progression systems work (package delivery, races, paintball combat)
6. How atmospheric effects are implemented (sky, clouds, god rays, aurora, fireflies, jellyfish)
7. How the server manages rooms, players, and game events
8. How the database schema stores player state, events, and lantern ledger
9. How VFX systems work (contrails, wake trails, particles, lens flare)
10. How the game handles mobile vs desktop controls
11. What patches are applied to Three.js internals and why
12. How to replicate these WebGL patterns in Rust

## Documentation Structure

| # | Document | Description |
|---|----------|-------------|
| 00 | Overview | What Tiny Skies is, project layout, technology stack |
| 01 | Architecture | Game engine structure, scene hierarchy, game loop |
| 02 | Terrain System | Simplex noise, terrain presets, surface generation, height maps |
| 03 | Flight Controls | Input handling, physics model, camera rig, touch controls |
| 04 | Vehicles | Biplane, carpet, void variants, boat — mesh creation, capabilities |
| 05 | Multiplayer Networking | Socket.IO protocol, state sync, room management, interpolation |
| 06 | Quest Systems | Package quest, landmark selfies, paintball combat, races |
| 07 | Progression & Upgrades | XP, levels, vehicle unlocks, upgrade manager |
| 08 | Atmospheric VFX | Sky presets, god rays, aurora, meteor showers, starfield, fireflies |
| 09 | Particle Systems | Contrails, wake trails, drift smoke, ring collect VFX, paintball splashes |
| 10 | NPC Systems | NPC planes, boats, sky jellyfish, gremlins, dialogue |
| 11 | Audio System | Spatial audio, ambient sounds, music |
| 12 | Server Architecture | Room management, flag system, event routes, paintball server logic |
| 13 | Database Schema | Prisma models, migrations, seed data, save feed |
| 14 | UI & HUD | Heads-up display, lobby, debug menu, overlays |
| 15 | Three.js Patches | Internal patches to Three.js for performance/compatibility |
| 16 | Deployment | Vercel, Railway, Docker, CI/CD |
| 17 | Rust Equivalents | How to replicate WebGL/game patterns in Rust |
| 18 | Production Patterns | Mobile optimization, performance, cross-platform |

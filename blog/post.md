# Bevy Jam 4

Hi. Recently I took part in [Bevy Jam 4](https://itch.io/jam/bevy-jam-4).
This is a game jam where you need to make a game using [Bevy](https://bevyengine.org) in 9 days.

Game jams are great - they are fun, let you test game ideas quickly, meet new people, help improve your skills, add a project to your portfolio.

This was my second time using bevy engine and I joined the jam to learn more about using the ECS approach, since usually I don't use it.

[The game we made can be played itch.io](https://kuviman.itch.io/ents).

[Source code is available on GitHub](https://github.com/kuviman/ents).

## The game idea

So, when the jam started, the theme was revealed - **That's a LOT of entities**.

After some brainstorming, I had a rough idea of a clicker / city builder game, where you automate harvesting resources and the goal is basically just getting more and more.

In the end, the player was to manage a city of crabs, with the end goal being building the bevy monument and starting the crabrave.

## Gameplay

The first couple days were spent on figuring out the gameplay, since the game idea I originally had was very rough.

There is a lot that can be included in a game like this, so I tried to carefully select the features that would make sense to implement in the scope of a game jam.

Here's a list of features we ended up with:

- houses, which spawn regular workers
- regular workers can carry 1 piece of a harvested resource in their inventiry
- resource storages, which workers need to go to after harvesting
- improved harvesters, which have improved inventory size of 10
- builders, which work on moving resources from storages to the build sites
- upgrade "academies", which turn regular workers into improved harvesters/builders
- upgraded buildings, which work like constructing the same building on top of an existing one
- roads, which make crabs move faster and are required to build something nearby
- the bevy monument, which after 3 upgrades starts the crab rave

In the first half of the jam, everything was rendered in 2d using rectangles of different colors.

## Infinite world gen

Even though was not really necessary in the end, I implemented the infinite world generation early.

The way it works is just spawning entities in big chunks - if a chunk of 64x64 is visible on the screen but was not generated yet, generate an corresponding event.

Other than for the sake of world generation, the chunks basically don't exist.

## Pathfinding

Doing pathfinding for a LOT of entities on an infinite map was a bit of a challenge.

Instead of doing pathfinding for every crab separately, I was calculating the way to the closest target for every tile instead.
In order to not take infinite amount of time, I only update up to a certain amount of tiles every frame,
and if a tile was indeed updated, also queued neighboring tiles for update.

In case some tile on the map was update, that tile was also queued for update with the highest priority.

This way the tiles that are closest to the recently updated tiles were updated sooner than those that are far away, but eventually the entire map would be calculated.

## Going 3d

After figuring out the gameplay in the first half of the jam, it was the time to make the game look prettier.

I have decided to try make it 3d, using simple box geometry for buildings and horizontal sprites for the crabs themselves.

It was pretty easy to do, and the performance was still great due to bevy's 0.12 update which included automatic instanced rendering of entities with same mesh/material.

## Fixing the web audio

When participating in a game jam, it is really important to have a working web build if you want more people to try out your game.

Unfortunately, bevy audio is not that great on the web.
Especially when you are making a game with a LOT of entities.

We tried both default `bevy_audio` and `bevy_kira_audio` and both were glitchy if there is any processing happening in the game.

So, on the last day of the jam I ended up building a plugin for bevy that instead uses my own `geng-audio` library, which uses existing higher level web apis for parsing and playing the audio like you would in JavaScript when targeting the web.

From what I understand, the issue is somewhere in `cpal`. The author of [Fyrox game engine](https://fyrox.rs) in [one of the updates](https://fyrox.rs/blog/post/fyrox-game-engine-0-30/) also moved to a custom audio library [tinyaudio](https://crates.io/crates/tinyaudio), claiming it fixes the sound artifacts.

## Thoughts on bevy after this jam

Last time I tried bevy was [for the previous bevy jam](https://kuviman.itch.io/linksider/devlog/520806/i-tried-bevy-for-the-first-time-for-a-game-jam).
I tried working on linksider more using bevy, but pretty quickly decided to rewrite in into my own custom engine.
Using ECS for a puzzle game being very error prone was the biggest reason (still maybe due to my little experience with it).

Using ECS for a game like we did this time, to the contrary, was pretty enjoyable despite some annoyances ([here's some notes](BEVY_POG.md)). I definetely want to continue trying to use ECS outside of a gamejam now.

I was especially interested in using ECS for the UI. Even though I'm not sure about bevy implementation (I only used it for like a day so far), but I think it allows splitting the layout/interaction/visual systems really nicely.

## See you next time

Not sure when will I use bevy next time, but I will most likely join the next bevy jam at least.

In case you are interested, I'm [streaming on twitch](https://twitch.tv/kuviman) almost every day programming in Rust, participating in game jams and working on [linksider](https://kuviman.itch.io/linksider) as the main project.

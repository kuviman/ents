# Bevy notes

This is a list of notes I was taking when participating in the bevy jam

## Rightclick context menu on the web

<https://github.com/bevyengine/bevy/issues/4721>

## Repeated textures

<https://github.com/bevyengine/bevy/issues/399#issuecomment-1794059638>

## AlphaMode::Blend breaks depth test

It disables depth test? Why?

## camera.viewport_to_world2d even though camera is 3d?

This API should not exist on 3d cameras

## Switching states & cleanup

remembering all entities and despawning seems very error prone

## State machine for an entity

Using enum component seems not extensible, I tried using `With<SpecificState>`

System conflicts - one system removes `StateA` and inserts `StateB`, another removes `StateA` and inserts `StateC`, need to be very careful.

## EntityCommands::insert_or_update

Entry API for components would be nice to have

## Bool component vs added/removed component

I'm assuming its faster to filter using `With<Component>` than `Component(bool)`, but the latter is sometimes easier to use, especially modify.
What about using both?

## Spawn entity with only type component -> populate other components pattern

When spawning entities, I only spawned them with an `EntType` enum component, and another system is populating other components necessary for gameplay mechanics/visuals.

Especially if I was splitting gameplay & visual component initialization into separate systems seems like a good pattern?

## mold

`mold` linker is faster but need to recompile from scratch after building for web

## compile times

better this time (~2 s), using dynamic linking, breaks sometimes, fix using `cargo clean -p my_game`.

## Game template not working

Web build had black screen - <https://github.com/NiklasEi/bevy_game_template/issues/84>

Native build also stopped working at some point. No idea what asset failed when using `bevy-asset-manager`.

## Has vs With

So I used `Has<Component>` instead of `With<Component>` as a filter in query and it took me 1 hour to find the problem.

## App does not exit

In logs:

```txt
bevy_window::system: No windows are open, exiting
```

In reality: need to Ctrl-C

## Inserting into despawned entities

Although entity existed during system? Becase of system order? Which is RNG? POG
But removing components of despawned entities does not panic?

## UI & cameras

Why is ui tied to camera if if uses pixels anyway?

## Clicking through buttons

<https://github.com/bevyengine/bevy/issues/3570>

And the proposed solution doesn't even work because of `Changed<Interaction>`.

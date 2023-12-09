# Bevy POG

## Repeated textures

<https://github.com/bevyengine/bevy/issues/399#issuecomment-1794059638>

## AlphaMode::Blend breaks depth test

WTF

## camera.viewport_to_world2d even though camera is 3d?

WTF

## Switching states & cleanup

remembering all entities and despawning seems hard

## State machine for a unit

System conflicts - one system removes StateA and inserts StateB, another removes StateA and inserts StateC

## EntityCommands::insert_or_update

## Bool component vs added/removed component

And what about both?

## spawn entity with only type component -> populate other components pattern

## mold

`mold` is faster but need to recompile from scratch after building for web

## compile times

better this time (~2 s), using dynamic linking, breaks sometimes, fix using `cargo clean -p my_game`.

## Game template not working

Web build had black screen - <https://github.com/NiklasEi/bevy_game_template/issues/84>

This shows how easy it is to break something when making changes?

Native build also stopped working at some point. No idea what asset failed when using `bevy-asset-manager`.

## Has vs With

So I used `Has<Component>` instead of `With<Component>` and it took me 1 hour to find the problem.

## App does not exit

In logs:

```txt
bevy_window::system: No windows are open, exiting
```

In reality: need to Ctrl-C

## Inserting into despawned entities

Although entity existed during system? Becase of system order? Which is RNG? POG
Also removing components seem to work

## UI & cameras

Why is ui tied to camera if if uses pixels anyway?

## Clicking through buttons

<https://github.com/bevyengine/bevy/issues/3570>

And the proposed solution doesn't even work because of `Changed<Interaction>`.

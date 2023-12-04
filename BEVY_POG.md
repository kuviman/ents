# Bevy POG

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

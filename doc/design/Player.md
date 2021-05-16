# Player entities

(Registered) players are entities which persist throughout a game, and which are
kept in a roster. Naturally, a roster entry needs to contain the player's name.
In addition, we choose to store the overall score in the entry as well.

As we need the ability to forcefully disconnect a player, we'll also make the
join handle associated to the player's connection task part of the entry. The
handle allows aborting a task, which will also tear down the connection. We may
also choose to include the player's peer address in order to provide this info
to the game master if necessary.

Ideally, an entry should allow querying whether a player is still active, i.e.
if the associated connection task is still running. Unfortunately, `tokio` does
not provide any ergonomic way for checking whether a task is running via a join
handle, and possible implementations for such a test from our side will not
scale particularly well (not that it matters with only few players), and come
with certain caveats.


## Player handle and tags

In the design for the [task level architecture](ServerClient.md) we established
the existence of a player handle owned (after registration) by the connection
task as well as player tags. As the player handle is created centrally and held
until a player disconnects, we can use it for notifying the central task of a
state change via an MPSC channel transporting player tags. A central instance
could then update the roster. I order to make such updates feasible, we will
wrap the join handles in an `Option`, with `None` indicating that the player is
no longer active.

In order to reduce the need for copying player data, we choose to keep entries
not in a list, but wrap each entry in a shared pointer (i.e. an `Arc`). The
associated player handle will contain an additional strong reference. Player
tags will also function as references, which will allow pointer-based comparison
and thus fast identification of a player.


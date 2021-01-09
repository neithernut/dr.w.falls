# Gameplay

Each tile of the 8x16 field may be either

 * free/unoccupied,
 * occupied by a virus (depicted as `><` or `--`) or
 * occupied by a capsule element (depicted as `()`).

Viruses and capsule elements are either red, yellow or blue. A tile can only be
occupied by a single virus or element at a given time.

At the beginning of a round, a number of viruses are distributed randomly on
the field. The distribution and colour of the individual viruses will be
identical for all players. Viruses will only be distributed among the 12 lower
rows of the field. Furthermore, viruses are never placed in configurations of
four or more viruses in a horizontal or vertical row.

A field will also be prepared for paused players.


## Falling capsule

A capsule consisting of two arbitrarily coloured elements spawns at the middle
of the top row (column 4 and 5). The capsule elements' colours are chosen
randomly, but the order of capsule colouring are identical for all players
during a round, i.e. all players will start a round with identical capsules.

The spawned capsule moves down one tile with each tick (a unit of time). The
player may move the capsule by providing input:

 * sending an `s` or `S` will move the capsule one tile to the left,
 * sending a  `d` or `D` will move the capsule one tile to the right,
 * sending a  `k` or `K` will rotate the capsule 90 degrees counter-clockwise,
 * sending an `l` or `L` will rotate the capsule 90 degrees clockwise and
 * sending a space (`0x20`) will move the capsule downwards one (additional)
   tile.

A movement is only accepted if the resulting positions of the capsule elements
would not conflict with pre-existing viruses or elements. In particular, the
capsule's movement downwards is halted if at least one of its elements would
then occupy the same tile as another capsule element or a virus.

Input driven movement of a capsule is decoupled from the tick-driven constant
movement downwards.

### Rotation

A rotation must not allow a player to move a capsule upwards nor accelerate the
movement downwards, i.e. a rotation must preserve the number of the lowest row
occupied by a capsule. A capsule is always rotated around one of its elements.


## Halting

If the downward movement of a capsule is halted, any vertical or horizontal rows
consisting of more than four capsule elements or viruses of the same colour will
be eliminated from the field. Any capsule or unbound/remaining capsule element
not supported by a capsule or virus moved downwards one tick at a time, without
any possibility for the player to interfere. Once their downward movement is
halted, the process of capsule (element) elimination and downward movement of
remnant element will repeat until no unsupported capsules or unbound elements
are left.

After this process ended, the player will receive all capsules (if any) sent by
other players, then gain control of a new capsule which will spawn at the top of
the field.

### Element binding

The unit of a capsule is preserved after its downward movement is halted and
only broken if one of its elements was eliminated.

### Sending capsule elements

If the process of repeated capsule elimination caused more than one row to be
removed, one capsule element is sent to other players for each row removed. The
capsule elements will be distributed based on the viruses remaining, with the
players with the fewest viruses remaining receiving capsules first. The sending
player will receive none of the capsule elements and any given player will
receive at most 4. A given player will only receive elements if all players with
fewer remaining viruses (excluding the sender) received the maximum number of
elements. The colour of the capsule elements sent will correspond to the colour
of the rows previously eliminated.

### Receiving capsule elements

After the process of elimination, the player will receive any outstanding
capsule elements in the units they were sent. If, for example, multiple
eliminations occurred which causing capsules to be sent to the player while
handling a single capsule, the player will receive the elements originating from
the first elimination process first, as a unit, then the elements originating
from the second and so on.

The received capsule elements will spawn at the top of the field at random
positions and move down one tile with each tick, without any possibility for the
player to move them around, until they are halted. After a capsule element is
halted, the capsule element elimination process described above will occur.


## Victory and Defeat

A player wins a round as soon as all viruses are eliminated from the field. A
player is defeated if any tile in the top row is occupied by a capsule element
(not including capsuled and elements which are in the process of moving
downwards). Upon defeat, the player will not gain the control of any new capsule
nor receive any capsule elements. The player has to wait for the round to end.

## Score

The round score is defined through the number of viruses remaining, with a lower
number indicating better performance. At the end of a round, that number is
added to the overall score. At the beginning of the first round, all players
start with an overall score of 0.


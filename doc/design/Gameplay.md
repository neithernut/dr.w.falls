# Gameplay implementation design

From the [gameplay specification](../Gameplay.md), we can derive the following
(additional) facts, which were not stated explicitly in that document:

 * In one way or another, almost always there's some capsule element falling,
   one tile with each tick. It's either the player-controlled capsule, unbound
   capsule elements or capsule elements not supported by a capsule or virus.
 * A settling capsule element always triggers the elimination of rows of four
   tiles of the same colour, which will only affect settled capsules. Since such
   a row must have been introduced through the settling (since all previous rows
   of four were removed), the settled element is part of the row.
 * At most one player controlled capsule will be active at any given point in
   time (for that player) and only if no received or unsupported elements are in
   motion.
 * The top row will never be occupied by settled capsule elements (since that is
   is the condition for defeat). Hence, we won't have to perform any such check
   when spawning a capsule or capsule element in the top row.
 * The downward movement of any capsule element is halted if it reaches the
   bottom of the field.
 * A capsule element or virus never changes colour.
 * A capsule element or virus is never duplicated, but only created, moved and
   eliminated.
 * Viruses never move (from one tile to another). Only capsules and capsule
   elements do.


## Supported/settled and unsupported capsules

The behaviour of unbound capsule elements received from another player and
capsules which lack support by another element or virus appears to be identical.
Hence, we can treat received capsule elements just like unsupported elements,
with the only difference being that they started out unsupported. Both the
settling mechanic and constant downwards movement is also identical for player
controlled capsules. Despite the controllability, we can thus even treat this
capsule as a mere (small) set of unsupported elements, which is subjected to
player-controlled transformations and may hence require a bit of additional
book-keeping. All capsules elements can thus be categorized as either supported
(or settled) ones, which will not move and be subject of element elimination,
and unsupported ones which move down one tile with each tick.


## Play field

The play field consists of (square) tiles, implying that capsules and viruses
always have integral positions. Functionally, the field may thus be considered a
map mapping positions to tile contents. In such a situation, there are two
commonly used options regarding the actual representation: an actual map or an
(two-dimensional) array of elements capable of representing the absence of
contents (i.e. a free tile). The former one usually is preferred if the field
is sparsely populated, which applies for us in some situations. However, with a
total of 128 tiles, the field's size is low enough for the use of a simple array
without the overhead of a map.

If we were to use one field for both settled and moving elements, we'd need to
locate and move each single unsettled element with each tick. Since we should
avoid scanning the entire field each tick, we'd want to represent this property
in some way. However, since all unsettled elements move down one tile at the
simultaneously, we have the option of using a separate, overlaying field for
unsettled capsule elements and use a different, time-varying index mapping for
that field.

On one hand, a capsule element from supported to unsupported or back would be a
move operation. On the other hand, a separate field would simplify implementing
the downward movement significantly while making this particular operation far
more efficient. Since only capsule elements will ever move, we'd also be able to
enforce this property through the item type. Thus, the option of using a
separate field for moving elements appears preferable. Furthermore, since any
capsule elements will become settled as soon as they reach the bottom row, we
can also use an array with 16 rows and simply use a modulo operation in the
mapping.

In order to keep the implementation of operations on fields intuitive, both the
field of settled elements and the field for moving elements will be implemented
as an `IndexMut` with an `Output` type representing a row of 8 tiles. In the
case of the field of moving elements, the trait implementation will contain the
offset defining the mapping, allowing the index operation to implement the
mapping transparently. Management of the offset will be abstracted behind a
member function.

During a round, the top row will never contain settled elements since this would
indicate defeat. This allows us to use an array of only 15 rows rather than 16
for the field of settled elements. In order to keep transfers between the fields
simple, the row indices should be kept identical. In addition, we'd like to have
the top row indexed as row 0, making a downward movement an increment operation
on the offset, and avoid negative numbers. Hence, we'll place the responsibility
of the mapping on the field of settled elements.


## Tiles

We'll use a simple value-less enum which allows comparison for representing the
three colours a virus or capsule element may have, as it's the obvious choice.
Viruses and capsules will be represented by non-copyable types each, which will
allow construction from a colour and querying, but not altering, that colour.
The capsule type will provide an indication whether the element is bound to
either the element above, below, left, or right to the element itself or none
at all. The binding will be modifiable via a member function.

For the field of settled elements and viruses, a tile will have the type of an
enum holding either a virus, a capsule element, or indicating that the tile is
free. For the field of moving elements, a tile will be an `Option` which may
hold a capsule.


## Detecting settling of capsule elements

A capsule element will halt or settle if the downward movement would either move
it past the lowest row or place it on an occupied tile. Hence, we'll perform the
detection of elements that need recategorization just before altering the row
offset for the field of moving elements. Firstly, the detection of elements in
the lowest row is trivial. For all other elements, we'll need to check whether
the tile in the field of settled elements just below a given element's mapped
position is occupied. If it is, the element in question and any element bound to
it settles.

The settling of an element may also cause the settling of any element directly
above. However, if we apply the detection described above sequentially from the
bottom row upwards, we'll automatically catch all of those. If we reach the top
row with elements to be settled, we have to declare defeat. We'll have to do so
before transferring elements since that row does not exist for the field of
settled elements.

After each settling, we'll perform the search and elimination of four or more
elements of the same colour, using the information that, if such a row exists,
at least one of the recently settled elements must be part of that row. Thus,
elements in the row above may not end up settling but progress downwards with
the next tick and settled elements may be recategorized as moving elements.


## Capsule elimination

The elimination process is performed on the field of settled elements only, on
basis of the hint described above. From the hinted position, we'll reach out in
each of the four directions, recording the number of tiles of the same colour,
stopping if we hit a free tile or a tile of another colour. If the sum of the
recorded tiles in either the vertical or horizontal directions is equal or
greater than four, we'll eliminate those tiles. If both happen to be greater
than four, the horizontal will have precedence.

The detection of those tiles will be encapsulated into a function taking a
reference to field and returning the set of tiles affected.


## Discovery of unsupported capsule elements

Since a capsule element's support will only vanish due to either elimination or
the recategorization of the supporting capsule as unsupported, we only have to
perform the discovery and recategorization of unsupported elements after an
elimination. Naturally, a given capsule element or virus will support only the
element occupying the tile directly above.

We could recursively detect and unsettle all relevant elements from a given
hint.  However, given the limited size of the field, we can also iterate from
the row affected by the elimination upwards, detecting capsules and unbound
elements in the field of settled elements which are not supported by elements
below them (any more) and moving them to the field of moving elements (thus
leaving capsule elements above it as unsupported).

Although this approach may appear be inefficient at first. However, it does
integrate well with the settling recategorization described above, which already
requires an iteration from the bottom to the top row. In order to simplify the
integration, we'll abstract the discovery of unsupported elements only on the
level of a single row (and relevant bound elements).


## Pre-tick functions

The settling, elimination, and discovery of unsupported capsule elements are
interdependent and operate only on the two fields. Hence, we should encapsulate
that logic into a single function. Since the settling process will detect the
defeat condition, the function will indicate defeat via its return type.

As described above, the function will settle any elements in the bottom row
first. Then it will perform a single pass over the rows, from the second row
from the bottom to the top. For each row, it will first perform the settling
required for that row and perform the elimination, followed by the discovery and
recategorization of any unsupported elements in that row. Since the latter will
only be required after an elimination, we'll gate this via a flag which will be
set upon elimination and cleared if no unsupported elements were discovered for
that row.

As any elimination event may also eliminate one or more viruses, we'll have to
determine the new virus count. For this purpose, we'll define a member function
of the field of settled elements returning the current number of viruses.


## Player controlled capsule

We need to spawn either a player controlled capsule or capsule elements received
from another player after the last moving element has settled, i.e. as soon as
the field of moving elements only contains free tiles. We could integrate such a
check in the settling and elimination logic described above, but this would
complicate things unnecessarily without any real benefit regarding the run-time.
Instead, we choose to implement this check as a member function of the moving
field.

If a player controlled capsule is present, it will be the only two elements
present on the field of moving elements. Thus, we could implement the player
controlled movement as a transformation on the entire field or locate the
capsule each time. However, even using vectorization this would be rather
excessive. We could store the position of the capsule. However, as we move its
elements, we'd need to perform considerable book-keeping.

Another practical solution is reducing the search space. We can arrange that at
least one of the capsule's elements is always part of the same unmapped row
which moved down with each tick. Since the capsule's elements are the only ones
moving, we can implement a "drop" by simply altering the offset. As long as we
rotate a capsule around one element which is in said row, the element itself
will also never move the row. Movements to the left or right will naturally also
preserve the elements' vertical position.

Still, we need to circumvent the mapping of the field of moving elements in some
way. We do this by defining a member function taking a mapped row number and
returning a handle identifying an unmapped row. This handle will be used for
querying the current mapped row number. When spawning a capsule, we'll retrieve
a handle for the top row and use it finding the capsule. In conjunction with
this handle, the bookkeeping necessary to track the capsule position will also
be reduced significantly. Although the search space for the capsule is already
reduced significantly, we thus decide to keep track of the horizontal position
through movements.

For the sake of decoupling, we'll use a value-less enum representing controlled
moves as input for the transformation. Voluntary drops will be an exception, as
this particular move may result in settling of the capsule. Furthermore, we'll
abstract the move logic into a function operating on references of the two
fields and a type encapsulating the row handle and horizontal position.
Construction of such a value will coincide with the spawning of a capsule. If
the capsule settles, we'll erase the handle and thus remove the possibility of
interference during phases in which the player should not have any control.
We'll achieve this by placing that value in an `Option`, which will be
initialized as we spawn a capsule and cleared whenever a capsule element
settles.


## Display updates

Ideally, we don't re-render the entire field with each change but only transmit
changes. Apart from the initial display of the initial field with viruses in
place, the only items that need to be updated within the field are moving and
eliminated items. The former is trivial, as we only have to erase elements from
the display as they are eliminated. For moving elements, we need to erase the
tile at a given capsule element's old position and draw the element at the new
position. We'll use this for player controlled horizontal movement and rotation.
However, for movement downwards, there exists a method which is more efficient
in some situations.

For downward movement, we'll iterate from the bottom row of the field of moving
elements to the top row. All elements in the given row will be drawn. For empty
tiles in that row, however, we'll erase that tile only if the tile directly
below it contains a capsule element. This way, we'll avoid erasing a tile just
for it to be filled afterwards, as such tiles are instead directly overwritten
in the display.

Naturally, the correct generation of display updates will be both critical for
user experience and somewhat intricate. We could use an abstraction which
combines any alteration on the field of moving elements with the generation of
updates. However, we'll also need to move elements from the field of settled
elements, for which we'd need to discard display updates, and generate updates
for elimination events, which do not involve the field of moving elements. Thus,
we decided against this approach despite its appeal regarding correctness.

Furthermore, we will use a type encapsulating the position within the displayed
field and colour, or its absence in the case of a freed tile, for updates. Each
event will result in the construction of an `IntoIterator` with those updates,
which will be passed to the display code for the generation and sending of
content. In particular, each of the movement functions will return such a list
of update items.


## Field preparation

During field preparation, the field is populated with a random distribution of
viruses, which is dictated by the central task. Values distributed through that
channel will be available as an immutable reference. As described above, we'd
like to have viruses and capsule elements non-copyable, which implies that a
field consisting of such elements would also be non-copyable. We could implement
`Clone` for the field, but this would either require considerable boilerplate
code or both the virus and capsule type to implement `Clone` too. This would
still be an option, since cloning is at least explicit and thus clearly visible.

Alternatively, we could also use another representation for distributing the
viruses and implement `From` for the field of settled elements. This option has
some appeal since the distributed field will only contain viruses, anyway.
Given that the initial field will be quite sparsely populated, we could use a
map mapping positions to colours for this purpose. Still, we'd want to re-use
the detection of four elements in a row we use for the elimination in order to
prevent configurations with four or more viruses of the same colour forming a
vertical or horizontal row.

This problem we can solve by introducing a trait requiring a function mapping
a position to a colour, and defining the detection function based on that trait.
The trait can then be implemented for both the field and the map used for
generating the distribution.


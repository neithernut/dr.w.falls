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

Both fields will internally be represented by lists of rows of 8 elements. We'll
define a generic type encapsulating one row as well as a column index type which
will only allow safe values. That index type will allow checked increment and
decrement. It appears natural to implement `std::iter::Step` for such a type, as
it would allow the use of ranges. However, that trait is marked as experimental
at the time of writing and thus we'll refrain from this for now. Instead, we'll
define a `DoubleEndedIterator` ourselves, although this will possibly based on
an `std::ops::Range`. Similarly, we'll define a dedicated row index type and
associated iterator.

The use of dedicated types for row and column indices without the possibility of
direct conversion will reduce the risk of accidentally confusing those indices.
In order to keep the implementation of operations on fields intuitive, both the
field of settled elements and the field for moving elements will be implemented
as an `IndexMut` allowing indexing based on a tuple of row and column indices,
with an `Output` type representing a tile. For more convenience, we'll also
define step functions on top of those tuples, which will allow stepping in the
four directions more conveniently.

The field of moving elements will contain the offset defining the mapping,
allowing the indexing to implement the mapping transparently. Management of the
offset will be abstracted behind a member function.

During a round, the top row will never contain settled elements since this would
indicate defeat. This would allows us to use an array of only 15 rows rather
than 16 for the field of settled elements. However, we want to allow the top row
to be occupied transiently: a capsule should be allowed to settle in the top row,
a player will be defeated only if that capsule is not eliminated directly
afterwards.


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


## Pre-tick functions

The settling, elimination, and discovery of unsupported capsule elements need to
be done just before altering the row offset for the field of moving elements.
Those operations are somewhat interdependent and operate only on the two fields.
It seems tempting to encapsulate that logic into a single function. However,
each of those stages will contribute differently to the overall gameplay logic.
Combining those different aspects within a single return value would make its
type rather complex, resulting in more boiler-plate at the call site.

In addition, we need to process the entire field(s) in each stage. Initially, we
assumed that we could perform the elimination as soon as an element settled.
However, elements which are not settled yet but would settle during the process
may also be part of the (extended) horizontal or vertical configuration to be
eliminated. If we eliminate rows-of-four prematurely, could miss elements of the
same colour in the same row or column.

Still, coupling those operations in some way does have some merit. Not only does
a data-dependency between the different stages exist, their order is also fixed
by the intended overall behaviour. We thus decide to model these constraints
through a cascade of different types. Each operation will be a function, which
will return a type containing the data for the next stage and may allow querying
some properties.

### Detecting settling of capsule elements

A capsule element will halt or settle if the downward movement would either move
it past the lowest row or place it on an occupied tile. Hence, we'll perform the
detection of elements that need recategorization just before altering the row
offset for the field of moving elements.

The detection of elements in the lowest row is trivial. For all other elements,
we'll need to check whether the tile in the field of settled elements just below
a given element's mapped position is occupied. If it is, the element in question
and any element bound to it settles.

The settling of an element may also cause the settling of any element directly
above. However, if we apply the detection described above sequentially from the
bottom row upwards, we'll automatically catch all of those.

Which elements settled is also relevant for elimination: any row-of-four in the
field resulting from the settling must have been introduced by the settling.
Hence, such a row must contain at least one recently settled element. Thus, we
collect the positions of all elements we transfer from the field of moving
elements to the field of settled elements. This data allows us to narrow down
the search for such rows considerably.

We'll encapsule the process of settling elements in a function which returns a
type encapsulating the positions of settled elements. In addition, the function
will take the row at which the settling process should be started, i.e. the
lowest row for which the process is performed, and return the first row in which
we did not settle all elements. This information should help reduce the amount
of computation we have to perform during this stage.

### Capsule elimination

The elimination process is performed on the field of settled elements only, on
basis of the hint described above. From any hinted position, we'll reach out in
each of the four directions, recording the number of tiles of the same colour,
stopping if we hit a free tile or a tile of another colour. If the sum of the
recorded tiles in either the vertical or horizontal directions is equal or
greater than four, we'll eliminate those tiles. If both happen to be greater
than four, the horizontal will have precedence.

The detection of those tiles will be encapsulated into a function taking a
reference to field and returning the set of tiles affected as well as the
row's colour. However, the entire elimination process will also be encapsulated
in a function. The function will return a type carrying information about the
eliminated rows. That information may be queried via a function returning:

    list of (colour, list of positions)

with each entry of that list will correspond to one eliminated configuration.

### Detection of defeat

We can determine whether or not to declare defeat by simply scanning the top row
in the field of settled elements. For this purpose, we'll define a function
taking as a parameter a reference to the field of settled elements. We could
enforce correct ordering by making it also take as parameter a reference of the
elimination function's return type, but there'd be little benefit.

### Virus count update

As any elimination event may also eliminate one or more viruses, we'll have to
determine the new virus count. Assuming the existence of a record of the initial
virus distribution and given the list of erased tiles from the elimination
process, we could determine the current virus count via some book-keeping.
However, given a record of the initial distribution, querying those positions
directly is far less complex and still far more efficient than a scan of the
entire field.

Still, we will only count viruses if an elimination event occurred. Detecting
this condition is trivial. Naturally, we'll define a function taking a reference
to the field of settled elements as well as the initial virus distribution.

### Discovery of unsupported capsule elements

Since a capsule element's support will only vanish due to either elimination or
the recategorization of the supporting capsule as unsupported, we only have to
perform the discovery and recategorization of unsupported elements after an
elimination. Naturally, a given capsule element or virus will support only the
element occupying the tile directly above.

The elimination function's return type already provides the positions of all
elements which were recently eliminated. From that we compute a set or list
holding positions of possibly unsupported elements by choosing the position
above. For each of the positions of that set, we determine whether an element
exists at that position and whether it still be supported via any element bound
to it. In this case, we must not consider any bound element above, since that
would otherwise naturally be supported by the element we initially considered.
Furthermore, we don't need to consider any element below since such an element
can't exist.

If the element is indeed unsupported, we move the element and any element bound
to it (including any bound element above) to the list of moving fields. In
addition, we record the positions above the moved elements in the list of
possibly unsupported elements.


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
We'll achieve this by placing that value in the variant of an enum which will be
initialized as we spawn a capsule. The second variant will indicate the presence
of uncontrolled capsules.


## Display updates

Ideally, we don't re-render the entire field with each change but only transmit
changes. Apart from the initial display of the initial field with viruses in
place, the only items that need to be updated within the field are moving and
eliminated items. The latter is trivial, as we only have to erase elements from
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

This problem we can solve by implementing `IndexMut` with out row and column
index types for the preparation field, and defining a trait which allows
querying the colour of a tile. The detection function can then be implemented
based on those two traits.


# Display

As described in the [interface specification](../Interface.md), display data
sent to a client will make use of ANSII escape sequences. In particular, escape
sequences for cursor positioning and colouring are of interest. Since the phases
differ mostly regarding the contents to display, we'll also want to clear the
entire screen during a transition. For aesthetic reasons, we'll also want to
hide the cursor during display updates.

As the player's terminal may be used after a session of the game, we should make
sure to leave it usable. In particular, we should reset the SGR parameters. In
addition, we should make sure the cursor ends up in a position which is not too
uncomfortable for the player. We'll choose the 24th row and 1st column for this
purpose and declare it a resting position. We will refrain to put any elements
in the 24th row or any row below it.

A player may leave the game by means which prevent us from issuing such a final
reset, i.e. through the termination of the connection. Thus, we choose to reset
the SGR parameters, return to the resting position and unhide the cursor after
each display update. Naturally, we'll want to make this automatic and not
clutter the display code with repeated explicit resets.

We naturally want to avoid the necessity of providing ANSII sequences as raw
bytes. Instead, we'll introduce a type representing draw commands as well as a
`tokio_util::codec::Encoder`, which will let us construct a sink for draw
commands. Such a draw command may indicate the intend to

 * clear the entire screen,
 * position the cursor at a declared absolute position,
 * set SGR parameters,
 * put characters/text on the screen at the cursor position or
 * show/hide the cursor.

We'll avoid exposing the draw commands to code not involved in drawing. Thus, we
may choose to wrap the sink into a type which will be somewhat opaque and allow
drawing code to retrieve a handle which will let it access the sink. Using the
RAII pattern, we can enforce (un)hiding and moving the cursor to the resting
position.


## General screen structure

The most important of the three phases naturally would be the round phase. The
corresponding screen features the field and the score board. Both are vertical
elements, meaning that they (usually) appear higher than wide, which lends
itself to a vertical split. Western tradition places the more important element,
which would be the field in this case, on the left. Hence, the score board would
be displayed in the right half. As the field will not fill the entire left half,
we can use the area below it for an indication of victory or defeat as well as
a "paused" indicator.

Although the displays differ the various phases, each of them features a display
of a roster or score board. We want to keep common elements in one place. Thus,
we'll reserve the screen's right half for the score board during other phases.

During the lobby and waiting phase, we'll require far less of the left half for
items relevant to the phase itself. We'll place that content in the upper half
and use the lower half for some minimal manual introducing the player to the
controls.

For realizing this structure, we'll use a data type representing an area of the
screen. An instance of this type will allow horizontal and vertical splitting as
well as placing an entity on the area. By using such an abstraction, we ensure
that we don't suffer from overlapping screen elements.

The display will provide a function for creating an area covering the entire
screen above the resting area, i.e. 80 columns and 23 rows. The function will
clear the screen before returning the area, thus ensuring that no old content
occupies the screen.

For entities which may be placed on the screen, we'll define a factory trait,
which will also be accepted by the function of the area type placing those
entities. The trait should provide a function for retrieving the size required
on the screen, a function creating the entity with a given position, as well as
a function for drawing the entity in its initial stage.

We'll need factories for the following entities:

 * Play field
 * Score boards
 * Static text (displayed once)
 * Dynamic text (some messages, waiting phase countdown, ...)
 * Input field (lobby)


## Play field

As described in the [gemeplay implementation design](Gameplay.md) the field is
updated based on tuples containing position and optional colour during a round.
These updates need to be translated into draw commands, which also involves a
coordinate transformation. Abstracting this generation behind a function taking
an update as parameter and returning a sequence of draw commands would allow
applying this function to the items of an `Iterator` over updates via `map`.
Since we'll always handle a list of update items anyway, we could as well
abstract the mapping operation itself behind a function.

In addition, to moving capsules, we'll also need to render the capsule preview
as well as the field outline and initial state, i.e. the initial distribution of
viruses. The latter requires the same coordinate transformation as the updates.
Hence, we'll abstract the transformation into a function which is called from
the initialization code and the function transforming updates into draw
commands.

Naturally, we'll define a datataype abstracting the field, which will features
public functions for the generation of draw commands for the initialization,
updates and the capsule preview. Instances of that type will be returned by an
appropriate entity factory.


## Score board/roster

Each phase features a roster or score-board, and while there are differences,
there also are lots of similarities between what's displayed as part of the
score-board. In particular, the overall score and player (connection) status
should be conveyed in the same way during the waiting and the round phases. An
abstraction which covers all these cases would be preferable.

In all cases, the roster will be exposed to connection tasks as a list, with
each item corresponding to one player, and be displayed as a table, with each
row corresponding to one list entry. Thus, we'll only need to focus on how to
accommodate for the different item types, assuming that we'll manage to agree on
a container or iterable.

We'll hide the differences between the item types behind a trait, which will
allow querying the name, the overall score, the player's connection status and
an optional additional item implementing `Display`. We'll use the latter for
rendering the readiness indicator for the waiting screen and the round score.

A generic function taking the list of items as input will render the entire
table. Rendering the entire table each time would, however, be unnecessary.
Instead we'll cache the previously rendered list inside a datatype, of which
the rendering function will be a member. This, of course, implies that the list
is (cheaply) clonable. Hence, it may be beneficial to transmit the roster
wrapped in an `Arc` or as some container for which cloning will be cheap. Since
no such container comes to mind, we'll wrap the container in an `Arc`.


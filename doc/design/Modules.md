# Modules

Naturally, the implementation of the software will be divided into modules. The
dependencies between those modules will pose a DAG, i.e. we'll avoid cyclic
dependencies between modules, even though they are allowed in rust. However, we
will allow sub-modules to depend on the parent module and vise versa --the
latter happens to be formally necessary when importing (from) siblings.


## Main module

The main or root module will contain all other modules as well as setup code.
Naturally, it will define the main function, which will parse the configuration,
setup listening sockets and start the game control task and game master console
task with those sockets.

For the parsing of command line arguments, we'll use [clap](https://clap.rs/),
since we're already somewhat accustomed to working with this library. The data
type holding the game configuration data will be defined in the main module, as
well as the public roaster, safeguarded by a lock allowing concurrent access by
the game control and game console tasks.


## Util module

Both the colour type and trait as well as the various index types defined in the
[gameplay implementation design](Gameplay.md) will likely be used in various
modules. Rather than having them all depend on the somewhat big gameplay module
described below, we'd rather have them in a separate module. Since none of them
really won't warrant a top-level module of their own, we throw them together.


## Gameplay module

This module will implement the remaining types and functions defined in the
[gameplay implementation design](Gameplay.md). The definition of the types and
functions themselves will be placed in submodules while the gameplay module
itself will only contain those module definitions and re-exports. We'll define:

 * an `items` submodule defining virus and capsule element types,
 * a private `row` submodule defining the internal row type,
 * a `static_field` submodule defining the field of settled elements as well as
   its tile type,
 * a `moving_field` submodule defining the field of moving elements,
 * a `tick` submodule defining the pre-tick functions and transfer types,
 * a `movement` submodule defining the movement function and input types and
 * a `preparation` submodule defining the preparation of a field.


## Display module

This module will implement all of the types and functions defined in the
[display design](Display.md). It will expose a type wrapping a `Sink` for
draw commands, which will allow keeping those commands private to the module,
the described functions and utility types for rendering the various elements as
well as some utility functions for displaying general text and clearing the
screen. In particular, it will expose the trait used for scoreboard entries,
which will be implemented by various types of the game module described below.

It will depend on at least the colour enum defined in the gameplay module.


## Game module

The game module implements and exports both the game control and the connection
tasks described in the [server-client architecture document](ServerClient.md) as
`async` functions. However, the phase functions for both of these tasks will be
implemented in phase-specific sub-modules:

 * the `lobby` module,
 * the `waiting` module and
 * the `round` module.

Each of these sub-modules may depend on the gameplay and display modules.

The game module itself will contain the definition of a generic type intended
for game-state updates, which depends on a type for actual update messages as
well as one for the transition message. This type will be instantiated in the
phase-specific modules and used in the signatures of the phase functions.

In addition, the game module will also contain the phase update channel's item
type definition and the phase control channel item type definition, which will
also be exposed. Furthermore, it will contain the definitions for the player
handle and tag.


## Console module

This module will implement the game master console task and expose it in the
form of an async function. Naturally, it will depend on the game module.


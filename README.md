# Dr. W. Falls

I'm a proponent of strong type systems and I fancy myself somewhat skilled in
dodging (run-time) program defects using constructive measurements. So I decided
to test the power of Rust's type system, and my skills, by writing a game
similar to [Dr. Mario](https://en.wikipedia.org/wiki/Dr._Mario) using a strict
waterfall method and recording any issue I encounter. Development will be done
in the following phases:

 * [X] Software requirements: we define the game mechanics and user interface,
       as detailed as feasible.
 * [ ] Analysis/Design: we define the modules and their scope.
 * [ ] Implementation: we implement each module. Compiling will be allowed, but
       running the program or any other form of dynamic test will not.
 * [ ] Testing: we write unit tests for the individual modules, run them, and
       fix any defect we find. However, we do not run the program itself or
       perform any test which would involve any form of user-interaction.
 * [ ] Operation: we do what game publisher do nowadays and let our customers
       find any remaining issues.


## Usage

The program recognized the following command line options:

 * `-l <addr>`, `--listen <addr>`: address to listen on
 * `-p <num>`, `--port <num>`: port to listen on for players
 * `--max-players <num>`: maximum number of players allowed
 * `--virs <num>`: number of viruses placed on the field at the beginning of a
   round
 * `--tick <num>`: duration of a tick (the time a capsule moved down one tile)
   in units of 100ms.
 * `--console`: spawn a game master console on standard input and output
 * `--console-sock <path>`: make a game master console accessible via the UNIX
   domain socket at the given path

When started, connecting players will enter a lobby first. The game must be
initiated by the game master or admin either via the game master console or by
sending SIGUSR1 to the process.


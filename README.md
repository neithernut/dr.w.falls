# Dr. W. Falls

I'm a proponent of strong type systems and I fancy myself somewhat skilled in
dodging (run-time) program defects using constructive measurements. So I decided
to test the power of Rust's type system, and my skills, by writing a game
similar to [Dr. Mario](https://en.wikipedia.org/wiki/Dr._Mario) using a strict
waterfall method and recording any issue I encounter. Development will be done
in the following phases:

 * [ ] Software requirements: we define the game mechanics and user interface,
       as detailed as feasible.
 * [ ] Analysis/Design: we define the modules and their scope.
 * [ ] Implementation: we implement each module. Compiling will be allowed, but
       running the program or any other form of dynamic test will not.
 * [ ] Testing: we write unit tests for the individual modules, run them, and
       fix any defect we find. However, we do not run the program itself or
       perform any test which would involve any form of user-interaction.
 * [ ] Operation: we do what game publisher do nowadays and let our customers
       find any remaining issues.


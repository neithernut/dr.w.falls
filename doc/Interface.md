# Player interface

Each player will access the game via a single TCP connection, via which the
program will receive inputs from the player in the form of US-ASCII and provide
a display of the game state under the assumption that the output is routed to an
ANSI-capable terminal displaying at least 80 columns and 25 lines. Furthermore,
the terminal is assumed to perform no input (line) buffering nor local echoing
of input.

Upon receiving an ETX (`0x03`) or EOT (`0x04`), the program will terminate the
connection.


## Lobby

Upon connection, the player is prompted for a name. The application will accept
any name with at least up to 16 printable ASCII characters including space
(`0x20` -- `0x7E`). During input mode:

 * all of those characters will be echoed back via the connection,
 * a backspace (`0x08`) will remove the last character from the name,
 * an LF (`0x0A`) or CR (`0x0D`) will terminate the input mode.

Upon termination of the input mode, any trailing whitespace is removed from the
name. This name is then registered as the player name associated with the
connection if the it isn't identical to any previously registered one. If the
registration fails, the player is presented an appropriate message and prompted
for a new name, i.e. the player will re-enter the input mode.

Once the registration was successful, the player is requested to wait for the
game master to start the game using an appropriate message.

While in the lobby, the player is presented the list of currently registered
names.


## Waiting screen

Before each round, each player will be presented a waiting screen. The screen
will present a countdown, as well as a scoreboard listing the players with their
names and overall score in tabular form. Each player may indicate readiness by
sending a printable character or space (`0x20` -- `0x7E`). The program will
transition to the game screen as soon as the countdown reached 0 or all players
have indicated readiness.


## Game screen

The user is presented with the play field consisting of 8 columns and 16 rows of
tiles, a display for the next capsule and the score-board. A single tile will
occupy two columns (and one row) of characters. The score-board will list all
players with their names, round and overall score in tabular form. For each
player, the row corresponding to the recipient will be highlighted.

A player will be able to provide input via the characters `s`, `d`, `k` and `l`,
both lower- and uppercase, and space (`0x20`). Upon receiving the character
`p` or the escape character (`0x1b`), the game will be paused for the individual
player (but continue for the others). Receiving any printable character or space
(`0x20` -- `0x7E`) over the connection will cause the game to be resumed for the
player.

Once a round ended, the program will transition to the waiting screen.


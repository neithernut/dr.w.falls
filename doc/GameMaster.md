# Game master console

The program provides a game master console. It may be used to alter various
settings and, more importantly, start the game after all players have joined.

The console accepts commands, which are terminated by either a newline or EOF.
Each command consists of the command word followed by an (optional) list of
arguments. The command word and arguments are separated by one or more
whitespace characters.


## Player control and introspection

The following commands may be used to control the registration of players.

 * `players`: print a list of the currently registered players with an
   enumeration and their current scores in tabular form. The player enumeration
   remains constant throughout the game, and may be used to refer to a player
   in other commands.
 * `accept [t[rue]|f[alse]]`: Control whether new registrations are accepted. If
   set to false, the program will not accept new connections and connections not
   associated with a registered player will be terminated. This command only has
   an effect during the lobby phase.
 * `restrict <num>`: restrict the number of registered players to the given
   value. If the number is reached, the program will stop accepting new
   registrations.
 * `kick <player num>`: Terminate the connection to the player with the given
   number. When issued during the lobby phase, the player is unregistered and
   the player name occupied (if any) becomes available again.

## Game control

The following commands may be used to start or end the game.

 * `status`: prints the current phase, that is "lobby", "waiting" or
   "round <number>".
 * `start`: this command starts the game, i.e. it ends the lobby phase. This
   implies `accept false`. Naturally, it has only an effect during the lobby
   phase.
 * `end`: this command ends the game and terminated the program. If it is issued
   during a round, it takes effect after that round.

## Settings

The following commands may be used to change the game settings. If issued during
a round, the settings will only take effect for the new round.

 * `set <property> <value>`: sets the given property to the given value.
 * `get <property>`: prints the value of the given property

The following settings are recognized:

 * `virs`: number of viruses with which a field is populated
 * `tick`: duration of a tick in units of 100ms.


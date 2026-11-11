# Privacy

This repository is public. A tracked file, a commit message, an issue, a pull request and a comment are all published the moment they are pushed or posted.

## Private

- **The audio.** It stays outside the repository and is never sent over a network.
- **Where things are on the owner's machine.** The path of a song, and any other path under the owner's home directory. Refer to the first-journey song as `MICROGROOVE_SONG`.
- **The owner's song materials other than the audio.** Lyrics, the directions a song was made from, and the files beside a song. From the owner's section annotation, only the times, the section names and the tempo are public. The program never reads any of these.

## Public

- **The first-journey song's title.** [The first journey](../foundation/03-first-journey.md) names it.

## Left out as a precaution

The owner has not ruled on these. Until the owner does, leave them out:

- how the owner makes songs: write "the written form the song was made from";
- what else is installed or running on the owner's machine: write "another program".

## The scan

`scripts/private-scan` fails when text holds a private term, says where, and never repeats the term. Its terms:

- the home directory and git's email, read from the machine;
- each path the main checkout's `.env` names, and the directory it is in;
- each line of the main checkout's `.private-terms`, a git-ignored file (see `.private-terms.example`). Add a term there when a private name or word is not covered above.

It runs by itself in two places:

- **Before a push**, over the messages and added lines of the commits being pushed (`.githooks/pre-push`).
- **Before `gh` posts**, over the text and the files of the call (`scripts/gh-private-scan`, a Claude Code hook). Pass a body as a file by its plain path: the hook stops a call whose file it cannot read, and inline text the shell would build.

By hand: `scripts/private-scan FILE...`, or pipe text into it.

The scan knows terms, not meaning. Lyrics or directions that are in no list pass it, so read what you are about to publish.

//! The guardrail scripts: the scan for private terms, the song loader and the hook on
//! `gh`. Each runs in a repository of its own with made-up terms, never in this one.
mod common;

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use common::scratch;

/// A made-up private term, and the made-up song path the repository's `.env` names.
const TERM: &str = "Quillfeather";
const SONG: &str = "/music/my songs/one.flac";

fn script(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join(name)
}

/// A program run in a directory, with a made-up home and with none of the variables by
/// which git or the owner's machine would point it at this repository or its song.
fn at(dir: &Path, program: &Path) -> Command {
    let mut command = Command::new(program);
    command.current_dir(dir).env("HOME", "/home/someone");
    for (name, _) in std::env::vars() {
        if name.starts_with("GIT_") || name.starts_with("MICROGROOVE_") {
            command.env_remove(name);
        }
    }
    command
}

fn git(dir: &Path, args: &[&str]) -> String {
    let identity = [
        "-c",
        "user.name=Someone",
        "-c",
        "user.email=someone@example.invalid",
    ];
    let output = at(dir, Path::new("git"))
        .args(identity)
        .args(args)
        .output()
        .unwrap();
    assert!(output.status.success(), "git {args:?}: {output:?}");
    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

/// Writes a file and commits it; the commit's abbreviated name.
fn commit(dir: &Path, file: &str, text: &str, message: &str) -> String {
    std::fs::write(dir.join(file), text).unwrap();
    git(dir, &["add", file]);
    git(dir, &["commit", "-q", "-m", message]);
    git(dir, &["rev-parse", "--short=9", "HEAD"])
}

/// A repository with one commit, an untracked `.env` that names a song, and one private term.
fn repository(name: &str) -> PathBuf {
    let dir = scratch(name);
    git(&dir, &["init", "-q", "-b", "main"]);
    std::fs::write(dir.join(".env"), format!("MICROGROOVE_SONG=\"{SONG}\"\n")).unwrap();
    std::fs::write(dir.join(".private-terms"), format!("# made up\n{TERM}\n")).unwrap();
    commit(
        &dir,
        ".gitignore",
        ".env\n.private-terms\n",
        "Ignore what is local",
    );
    dir
}

fn scan(dir: &Path, args: &[&str], input: &str) -> Output {
    let mut child = at(dir, &script("private-scan"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

/// What a scan said, having found something. It must never repeat the term it found.
fn finding(output: &Output) -> String {
    let said = String::from_utf8_lossy(&output.stdout).to_string();
    assert!(!output.status.success(), "expected a finding: {output:?}");
    let lower = said.to_lowercase();
    assert!(
        !lower.contains(&TERM.to_lowercase()) && !lower.contains("my songs"),
        "a finding must not repeat the term: {said}"
    );
    said
}

#[test]
fn the_scan_passes_clean_text_and_says_where_a_term_is_without_repeating_it() {
    let dir = repository("guardrails-scan");
    std::fs::write(dir.join("clean.md"), "nothing private\nat all\n").unwrap();
    let clean = scan(&dir, &["clean.md"], "");
    assert!(
        clean.status.success() && clean.stdout.is_empty(),
        "{clean:?}"
    );

    // A term is found whatever its case.
    let text = format!("one\ntwo\nabout {} here\n", TERM.to_lowercase());
    std::fs::write(dir.join("note.md"), text).unwrap();
    let said = finding(&scan(&dir, &["clean.md", "note.md"], ""));
    assert!(said.contains("note.md, line 3"), "{said}");

    // With no file named it reads standard input. The song's path, the directory the
    // song is in and any path under the home directory are terms too.
    for private in [SONG, "/music/my songs/lyrics.md", "/home/someone/notes.txt"] {
        finding(&scan(&dir, &[], &format!("see {private}\n")));
    }
    assert!(
        scan(&dir, &[], "see /music/other/two.flac\n")
            .status
            .success()
    );
}

#[test]
fn the_scan_of_commits_reads_messages_and_added_lines_and_not_removed_ones() {
    let dir = repository("guardrails-commits");
    let first = git(&dir, &["rev-parse", "HEAD"]);
    let adds = commit(&dir, "note.md", &format!("about {TERM}\n"), "Add a note");
    let removes = commit(&dir, "note.md", "about nothing\n", "Take it out again");
    let says = commit(&dir, "other.md", "fine\n", &format!("Thank {TERM}"));
    let clean = commit(&dir, "other.md", "still fine\n", "A clean one");

    let all = format!("{first}..HEAD");
    let said = finding(&scan(&dir, &["--commits", &all], ""));
    let named: Vec<&str> = [&adds, &removes, &says, &clean]
        .into_iter()
        .filter(|commit| said.contains(commit.as_str()))
        .map(String::as_str)
        .collect();
    assert_eq!(named, [adds.as_str(), says.as_str()], "{said}");

    // What a push of only the last commit would send is clean.
    let last = format!("{says}..HEAD");
    assert!(scan(&dir, &["--commits", &last], "").status.success());
}

#[test]
fn the_song_loader_sets_the_song_in_a_worktree_from_the_main_checkout() {
    let dir = repository("guardrails-song");
    let worktree = scratch("guardrails-song-worktree");
    std::fs::remove_dir(&worktree).unwrap();
    git(&dir, &["worktree", "add", "-q", worktree.to_str().unwrap()]);
    assert!(!worktree.join(".env").exists());

    let print = ["sh", "-c", "printf %s \"$MICROGROOVE_SONG\""];
    let output = at(&worktree, &script("with-song"))
        .args(print)
        .output()
        .unwrap();
    assert_eq!(String::from_utf8_lossy(&output.stdout), SONG, "{output:?}");

    // Without a `.env` it says so plainly and runs nothing.
    std::fs::remove_file(dir.join(".env")).unwrap();
    let output = at(&worktree, &script("with-song"))
        .args(print)
        .output()
        .unwrap();
    let message = common::complaint(&output);
    assert!(
        message.contains(".env") && output.stdout.is_empty(),
        "{message}"
    );
}

/// What the hook on `gh` does with a command an agent is about to run: its exit status,
/// 0 to let it run and 2 to stop it, and what it said.
fn hook(dir: &Path, command: &str) -> (i32, String) {
    let call = serde_json::json!({
        "tool_name": "Bash",
        "cwd": dir,
        "tool_input": { "command": command },
    });
    let mut child = at(dir, &script("gh-private-scan"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let text = call.to_string();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(text.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    let said = String::from_utf8_lossy(&output.stderr).to_string();
    assert!(
        !said.to_lowercase().contains(&TERM.to_lowercase()),
        "the hook must not repeat the term: {said}"
    );
    (output.status.code().unwrap(), said)
}

#[test]
fn the_hook_on_gh_lets_clean_posts_run_and_stops_what_holds_a_term_or_cannot_be_read() {
    let dir = repository("guardrails-hook");
    std::fs::write(dir.join("clean.md"), "## Report\n\nnothing private\n").unwrap();
    std::fs::write(
        dir.join("private.md"),
        format!("## Report\n\nabout {TERM}\n"),
    )
    .unwrap();
    let here = dir.to_str().unwrap();

    let runs = [
        "ls -la".to_string(),
        "gh issue view 1 --json body --jq .body".to_string(),
        "gh api repos/someone/something/issues/1/comments --paginate --jq '.[].id'".to_string(),
        "gh issue comment 1 --body-file clean.md".to_string(),
        // A path built from a variable set in the same command is followed.
        format!("S={here}; cd /; gh issue comment 1 --body-file $S/clean.md | tail -1"),
        // Another program's options are not taken for gh's.
        format!("grep -c -i -F -e {TERM} private.md; gh pr edit 5 --body-file clean.md"),
        "gh pr create --title 'A clean title' --body-file clean.md".to_string(),
        "gh issue comment 1 --body 'nothing private'".to_string(),
    ];
    for command in &runs {
        let (status, said) = hook(&dir, command);
        assert_eq!(status, 0, "{command}: {said}");
    }

    let stopped = [
        "gh issue comment 1 --body-file private.md".to_string(),
        format!(
            "gh api -X PATCH repos/someone/something/issues/comments/1 -F body=@{here}/private.md"
        ),
        format!("gh pr create --title 'About {TERM}' --body-file clean.md"),
        format!(
            "gh issue comment 1 --body \"about {}\"",
            TERM.to_uppercase()
        ),
        // A call on a later line, and one carried over a line break.
        "gh issue view 1 --json body\ngh issue comment 1 --body-file private.md".to_string(),
        "gh issue comment 1 \\\n  --body-file private.md".to_string(),
        // What it cannot read it does not let through.
        "gh issue comment 1 --body-file $NOWHERE/clean.md".to_string(),
        "gh issue comment 1 --body-file missing.md".to_string(),
        "cat private.md | gh issue comment 1 --body-file -".to_string(),
        "gh issue comment 1 --body \"$(cat private.md)\"".to_string(),
    ];
    for command in &stopped {
        let (status, said) = hook(&dir, command);
        assert_eq!(status, 2, "{command}: {said}");
        assert!(said.contains("privacy.md"), "{command}: {said}");
    }
}

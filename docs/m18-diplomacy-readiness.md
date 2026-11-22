# Milestone 18 diplomacy candidate

[#199](https://github.com/diegoami/totengine/issues/199) owns the legal
engine decisions and [#200](https://github.com/diegoami/totengine/issues/200)
owns the shared client route. The bounded play path starts from an authored
scenario with a recorded contact at war. The source scenario can be loaded
through experimental entry or through its engine checkpoint.

Open **Known contacts** from Advisors or F3. Inspect the relationship, then
select **Talk to civ 2** and **Review peace proposal**. Escape goes back to
topics. Save the proposal with **Save pending choice**, send it explicitly,
then reload the pending checkpoint and send it again. Use **Return to
contacts** to see the recorded relationship after the response. Perform the
walkthrough with both the accepting and refusing fixture on each client.

The engine uses the declared `engine-peace-v1` response policy. The controlled
fixtures start at the two sides of its threshold and produce a recorded treaty
or an unchanged world. Recorded peace uses the existing bilateral treaty
setter. The route covers one unconditional proposal, with native AI response
parity, automatic encounter scheduling, advanced exchanges, intelligence
details, and combat enforcement outside its scope. See the
[screen contract](ui/screen-contracts.md#diplomacy-and-intelligence) and
[native boundary evidence](evidence/diplomacy-peace.json).

On a schema-2 experimental import, accepted peace starts the declared engine
observation continuation. Preview, Back, Leave and refusal retain the saved
initial view. A schema-2 regression verifies the transition and compares the
restored view and checkpoint with uninterrupted play.

## Reproduce

Use a private install copy containing the root and Original text and image
resources. Point `CIVTOT_HOME` at the copy for the real-input walkthrough.
The ignored fixture author writes two new authored SAV files under Original
and two starting engine checkpoints in a separate scratch folder.

```sh
export CIVTOT_HOME=/path/to/private/install
export TOTENGINE_PEACE_OUT=/path/to/private/peace-fixtures
cargo test -p totui --lib installed_peace_input_fixture -- --ignored
export TOTENGINE_PEACE_FIXTURES="$TOTENGINE_PEACE_OUT"
cargo build -p totui --features desktop --bins
python3 tools/check_client_linux.py
```

For Chromium, give the browser host a separate scratch `TOTENGINE_SAVES`
directory, start `totplay --no-open --port=8840`, and run
`tools/check_client_browser.cjs` with `TOTENGINE_CLIENT_ORIGIN` set to that
loopback URL, `TOTENGINE_CLIENT_SAVES` set to the host's save folder, and
`TOTENGINE_PEACE_FIXTURES` unchanged. Both scripts compare the uninterrupted
and resumed checkpoints after acceptance and refusal.
All six pending, direct and resumed checkpoint comparisons matched between
Chromium and Linux SDL. The [input evidence](evidence/diplomacy-peace-input.json)
records the fixture and checkpoint hashes. Windows desktop compilation was
checked separately.

For the native evidence, use the original installation and the Unicorn
environment in [environment.md](environment.md). Run
`tools/diplomacy_peace_check.py --out` to a fresh file and compare its bytes
with the retained report.

Owner acceptance of the new route on both clients and the owner-launched
independent review are separate release gates. This candidate has no assigned
version or tag.

use serde::Serialize;
use serde_json::json;
use serde_json::Value::{self as Json, Null};

use super::*;

// Parsing helpers

fn try_parse(input: &str, disable_xpose: bool) -> Result<Vec<Song>> {
    let mut songs = vec![];
    let mut parser = Parser::new(input, "<test>".into(), ParserConfig::default());
    parser.set_xp_disabled(disable_xpose);
    parser.parse(&mut songs)?;
    Ok(songs)
}

fn parse(input: &str, disable_xpose: bool) -> Vec<Song> {
    try_parse(input, disable_xpose).unwrap()
}

fn parse_one(input: &str) -> Song {
    let mut songs = parse(input, false);
    assert_eq!(songs.len(), 1);
    let song = songs.drain(..).next().unwrap();
    song
}

fn parse_one_para(input: &str) -> Paragraph {
    let blocks = parse_one(input).blocks;
    let block = Vec::from(blocks).drain(..).next().unwrap();
    match block {
        Block::Verse(v) => Vec::from(v.paragraphs).drain(..).next().unwrap(),
        _ => panic!("First block in this Song isn't a Verse"),
    }
}

fn get_verse(song: &Song, block_num: usize) -> &Verse {
    match &song.blocks[block_num] {
        Block::Verse(verse) => verse,
        b => panic!("Unexpected block type: {:?}", b),
    }
}

// AST helpers
// These helpers purposefully return JSON so that we verify the schema as well.

fn song(
    title: impl AsRef<str>,
    subtitles: impl IntoIterator<Item = &'static str>,
    notation: impl AsRef<str>,
    blocks: impl IntoIterator<Item = Json>,
) -> Json {
    let subtitles: Vec<_> = subtitles
        .into_iter()
        .map(|s| Json::String(s.into()))
        .collect();

    json!({
        "title": title.as_ref(),
        "subtitles": subtitles,
        "notation": notation.as_ref(),
        "blocks": blocks.into_iter().collect::<Vec<_>>(),
    })
}

fn b_verse(
    typ: &str,
    label: impl Serialize,
    paras: impl IntoIterator<Item = impl IntoIterator<Item = Json>>,
) -> Json {
    let paras: Vec<Vec<Json>> = paras
        .into_iter()
        .map(|inlines| inlines.into_iter().collect())
        .collect();

    json!({
        "type": "b-verse",
        "label": { typ: label },
        "paragraphs": paras,
    })
}

fn ver_verse(label: u32, paras: impl IntoIterator<Item = impl IntoIterator<Item = Json>>) -> Json {
    b_verse("verse", label, paras)
}

fn ver_chorus(
    label: impl Serialize,
    paras: impl IntoIterator<Item = impl IntoIterator<Item = Json>>,
) -> Json {
    b_verse("chorus", label, paras)
}

fn ver_custom(
    label: &str,
    paras: impl IntoIterator<Item = impl IntoIterator<Item = Json>>,
) -> Json {
    b_verse("custom", label, paras)
}

fn ver_none(paras: impl IntoIterator<Item = impl IntoIterator<Item = Json>>) -> Json {
    b_verse("none", json!({}), paras)
}

fn i_text(text: impl AsRef<str>) -> Json {
    json!({ "type": "i-text", "text": text.as_ref() })
}

fn i_break() -> Json {
    json!({ "type": "i-break" })
}

fn i_chord(
    chord: &str,
    alt_chord: impl Serialize,
    backticks: u32,
    inlines: impl IntoIterator<Item = Json>,
) -> Json {
    json!({
        "type": "i-chord",
        "chord": chord,
        "alt_chord": alt_chord,
        "backticks": backticks,
        "inlines": inlines.into_iter().collect::<Vec<_>>(),
    })
}

fn i_strong(inlines: impl IntoIterator<Item = Json>) -> Json {
    json!({ "type": "i-strong", "inlines": inlines.into_iter().collect::<Vec<_>>() })
}

fn i_emph(inlines: impl IntoIterator<Item = Json>) -> Json {
    json!({ "type": "i-emph", "inlines": inlines.into_iter().collect::<Vec<_>>() })
}

fn i_xpose(typ: &str, value: impl Serialize) -> Json {
    json!({ "type": "i-transpose", typ: value })
}

fn i_chorus_ref(num: impl Serialize, prefix_space: &str) -> Json {
    json!({ "type": "i-chorus-ref", "num": num, "prefix_space": prefix_space })
}

#[test]
fn songs_split() {
    let input = r#"
No-heading lyrics
# Song 1
Lyrics lyrics...
# Song 2
Lyrics lyrics...
    "#;

    let songs = parse(&input, false);

    assert_eq!(songs.len(), 3);
    assert_eq!(&*songs[0].title, FALLBACK_TITLE);
    assert_eq!(&*songs[1].title, "Song 1");
    assert_eq!(&*songs[2].title, "Song 2");
}

#[test]
fn ast_split_at() {
    let input = r#"_text **strong** `C`text2 **strong2**_"#;

    let arena = Arena::new();
    let options = ComrakOptions::default();
    let root = comrak::parse_document(&arena, input, &options);

    let para = root.children().next().unwrap();
    let em = para.children().next().unwrap();
    let code = em.split_at(3, &arena);
    let em2 = code.split_at(1, &arena);

    assert_eq!(em.children().count(), 3);
    assert_eq!(em.as_plaintext(), "text strong ");
    assert_eq!(code.children().count(), 1);
    assert_eq!(code.as_plaintext(), "C");
    assert_eq!(em2.children().count(), 2);
    assert_eq!(em2.as_plaintext(), "text2 strong2");
}

#[test]
fn ast_preprocess() {
    let input = r#"
Lyrics _em **strong `C` strong**
em_ lyrics
    "#;

    let arena = Arena::new();
    let options = ComrakOptions::default();
    let root = comrak::parse_document(&arena, input, &options);

    let para = root.children().next().unwrap();
    para.preprocess(&arena);

    assert_eq!(para.children().count(), 7);
    let code = para
        .children()
        .find(|c| c.is_code())
        .unwrap()
        .as_plaintext();
    assert_eq!(code, "C");
    para.children().find(|c| c.is_break()).unwrap();
}

#[test]
fn parse_verses_basic() {
    let input = r#"
# Song
1. First verse.

Second paragraph of the first verse.

2. Second verse.

Second paragraph of the second verse.

3. Third verse.
4. Fourth verse.
> Chorus.
"#;

    parse_one(input).assert_eq(song(
        "Song",
        [],
        "english",
        [
            ver_verse(
                1,
                [
                    [i_text("First verse.")],
                    [i_text("Second paragraph of the first verse.")],
                ],
            ),
            ver_verse(
                2,
                [
                    [i_text("Second verse.")],
                    [i_text("Second paragraph of the second verse.")],
                ],
            ),
            ver_verse(3, [[i_text("Third verse.")]]),
            ver_verse(4, [[i_text("Fourth verse.")]]),
            ver_chorus(Null, [[i_text("Chorus.")]]),
        ],
    ));
}

#[test]
fn parse_verses_corners() {
    let input = r#"
# Song

Verse without any label.

Next paragraph of that verse.

### Custom label

Lyrics Lyrics lyrics.

> Chorus 1.
>> Chorus 2.
>
> Chorus 1 again.
>
> More lyrics.

Yet more lyrics (these should go to the chorus as well actually).

>>> Chorus 3.

More lyrics to the chorus 3.

"#;

    parse_one(input).assert_eq(song(
        "Song",
        [],
        "english",
        [
            ver_none([
                [i_text("Verse without any label.")],
                [i_text("Next paragraph of that verse.")],
            ]),
            ver_custom("Custom label", [[i_text("Lyrics Lyrics lyrics.")]]),
            ver_chorus(1, [[i_text("Chorus 1.")]]),
            ver_chorus(2, [[i_text("Chorus 2.")]]),
            ver_chorus(
                1,
                [
                    [i_text("Chorus 1 again.")],
                    [i_text("More lyrics.")],
                    [i_text(
                        "Yet more lyrics (these should go to the chorus as well actually).",
                    )],
                ],
            ),
            ver_chorus(
                3,
                [
                    [i_text("Chorus 3.")],
                    [i_text("More lyrics to the chorus 3.")],
                ],
            ),
        ],
    ));
}

#[test]
fn parse_subtitles() {
    let input = r#"
# Song
## Subtitle 1
## Subtitle 2

Some lyrics.

## This one should be ignored
"#;

    let song = parse_one(input);
    assert_eq!(
        &*song.subtitles,
        &["Subtitle 1".into(), "Subtitle 2".into(),]
    );
}

#[test]
fn parse_chords() {
    let input = r#"
# Song
1. Sailing round `G`the ocean,
Sailing round the ```D```sea.
"#;
    parse_one_para(input).assert_eq(json!([
        i_text("Sailing round "),
        i_chord("G", Null, 1, [i_text("the ocean,")]),
        i_break(),
        i_text("Sailing round the "),
        i_chord("D", Null, 3, [i_text("sea.")]),
    ]));
}

#[test]
fn parse_inlines() {
    let input = r#"
# Song
1. Sailing **round `G`the _ocean,
Sailing_ round the `D`sea.**
"#;
    parse_one_para(input).assert_eq(json!([
        i_text("Sailing "),
        i_strong([i_text("round ")]),
        i_chord(
            "G",
            Null,
            1,
            [i_strong([i_text("the "), i_emph([i_text("ocean,")])])]
        ),
        i_break(),
        i_strong([i_emph([i_text("Sailing")]), i_text(" round the "),]),
        i_chord("D", Null, 1, [i_strong([i_text("sea.")])]),
    ]));
}

#[test]
fn parse_extensions() {
    let input = r#"
# Song

!+5
!!czech

> Chorus.

1. Lyrics !!> !!!english !+0
!+2 More lyrics !>

# Song two

> Chorus.

>> Chorus two.

1. Reference both: !> !>>
!> First on the line.
Mixed !>> in text.

"#;

    let songs = parse(input, true);

    songs[0].blocks.assert_eq(json!([
        ver_none([[
            i_xpose("t-transpose", 5),
            i_break(),
            i_xpose("t-alt-notation", "german")
        ],]),
        ver_chorus(Null, [[i_text("Chorus.")]]),
        ver_verse(
            1,
            [[
                i_text("Lyrics !!> !!!english"),
                i_xpose("t-transpose", 0),
                i_break(),
                i_xpose("t-transpose", 2),
                i_text(" More lyrics"),
                i_chorus_ref(Null, " "),
            ],]
        ),
    ]));

    songs[1].blocks.assert_eq(json!([
        ver_chorus(1, [[i_text("Chorus.")]]),
        ver_chorus(2, [[i_text("Chorus two.")]]),
        ver_verse(
            1,
            [[
                i_text("Reference both:"),
                i_chorus_ref(1, " "),
                i_chorus_ref(2, " "),
                i_break(),
                i_chorus_ref(1, ""),
                i_text(" First on the line."),
                i_break(),
                i_text("Mixed"),
                i_chorus_ref(2, " "),
                i_text(" in text."),
            ]]
        ),
    ]));
}

#[test]
fn transposition() {
    let input = r#"
# Song

!+5
!!czech

> 1. `Bm`Yippie yea `D`oh! !+0
!+0 Yippie yea `Bm`yay!

"#;

    let song = parse_one(input);
    song.blocks.assert_eq(json!([ver_chorus(
        Null,
        [[
            i_chord("Em", "Hm", 1, [i_text("Yippie yea ")]),
            i_chord("G", "D", 1, [i_text("oh!")]),
            i_break(),
            i_text("Yippie yea "),
            i_chord("Bm", "Hm", 1, [i_text("yay!")]),
        ]]
    )]));
}

#[test]
fn transposition_error() {
    let input = r#"
# Song

!+5

> 1. `Bm`Yippie yea `D`oh!
Yippie yea `X`yay!
"#;

    let err = try_parse(input, false).unwrap_err();
    assert_eq!(err.file, "<test>");
    assert_eq!(err.line, 7);
    assert_eq!(err.kind, ErrorKind::Transposition { chord: "X".into() });
}

#[test]
fn parse_verse_numbering() {
    let input = r#"
# Song 1

1. Verse 1.
> Chorus 1.
1. Verse 2.
>> Chorus 2.
1. Verse 3.

# Song 2

1. Verse 1.
2. Verse 2.
> Chorus.
>> Chorus two.
3. Verse 3.
3. Verse 3.
"#;

    let songs = parse(input, true);

    assert_eq!(get_verse(&songs[0], 0).label, VerseLabel::Verse(1));
    assert_eq!(get_verse(&songs[0], 2).label, VerseLabel::Verse(2));
    assert_eq!(get_verse(&songs[0], 4).label, VerseLabel::Verse(3));

    assert_eq!(get_verse(&songs[1], 0).label, VerseLabel::Verse(1));
    assert_eq!(get_verse(&songs[1], 1).label, VerseLabel::Verse(2));
    assert_eq!(get_verse(&songs[1], 4).label, VerseLabel::Verse(3));
    assert_eq!(get_verse(&songs[1], 5).label, VerseLabel::Verse(4));
}

// FIXME: test HTML

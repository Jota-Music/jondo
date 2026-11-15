use std::collections::HashSet;

use crate::shared::music::domain::{Song, Video};

use super::Youtube;

#[cfg(test)]
#[path = "ranking.test.rs"]
mod tests;

// Penalty weights for ranking search results. A missing title token outweighs
// any duration gap on purpose: a wrong song is a worse failure than a wrong
// version, and a live cut can land within 2% of the studio duration, so
// duration alone can never prove two tracks are the same song.
//
// The title and duration signals are 0 for a perfect match, so the uploader
// takes negative weights to break the ties they leave behind, and the version
// signal takes a positive weight on top of whatever gap it already has.
const TOKEN_PENALTY: f64 = 200.0;
const DURATION_WEIGHT: f64 = 100.0;
const DURATION_CAP: f64 = 150.0;
const KEYWORD_PENALTY: f64 = 30.0;
const KEYWORD_CAP: f64 = 90.0;

// Uploader tiers, best first. The artist posting their own release is the
// master; a "- Topic" channel restates the record with no editor's cuts and no
// filler; a random channel is nothing in particular; a karaoke or subtitled
// re-upload is described by its video, not the track, so its running time is
// the video's and not the track's and it looks like a perfect length match.
//
// A topic channel sits below the artist's own because a channel named after an
// artist can be impersonated, while a "- Topic" channel is one YouTube opened
// itself. The lyric tier is the only one that has to clear a real duration gap
// to be harmless, which is why it outweighs the topic bonus. The topic tier must
// stay below the credit bonus by more than a full duration gap, so naming the
// artist in a title can never tie the official upload.
const CHANNEL_BONUS: f64 = -30.0;
const TOPIC_BONUS: f64 = -28.0;
const CREDIT_BONUS: f64 = -20.0;
const LYRIC_PENALTY: f64 = 40.0;

// How far the uploader's trust survives a length disagreement. Within TIER_FULL
// the tier applies whole; by TIER_GONE the length is so far off that the uploader
// says nothing about it, and the bonus fades in between.
const TIER_FULL: f64 = 0.05;
const TIER_GONE: f64 = 0.15;

// What YouTube appends to the channel it auto-opens for an artist. It localises
// that suffix, so the list cannot be complete; clientContext pins the search
// language to English, and a topic channel that slips past this list is merely
// unranked rather than misfiled.
const TOPIC_SUFFIXES: [&str; 3] = ["- topic", "- tema", "- temas"];

// What YouTube and the labels put after the artist's name on a channel the
// artist really owns. Anything else carrying the artist's name is someone
// else's: "Trueno and MILO J", "Bizarrap and Daddy Yankee" or "WOS DS3" all
// begin with the name and none of them are the artist.
const OFFICIAL_SUFFIXES: [&str; 3] = ["official", "oficial", "vevo"];

// Words that mark a video as something other than the studio master: a live
// cut, a reworked or degraded copy. They are matched against the already
// normalised title, so "Live" reads as "live" while "Live It Up" keeps all of
// its words.
//
// "cover" is deliberately absent. Every other word describes a derivative, which
// by definition is not the first release, but an artist who releases a cover as
// a single owns the master of it, and labels name that upload "(Cover Audio)" on
// the artist's own VEVO channel.
const VERSIONS: [&str; 35] = [
    "live",
    "vivo",
    "directo",
    "concert",
    "concierto",
    "unplugged",
    "acoustic",
    "acustico",
    "karaoke",
    "instrumental",
    "orchestral",
    "remix",
    "edit",
    "bootleg",
    "mashup",
    "medley",
    "slowed",
    "sped",
    "nightcore",
    "reverb",
    "reversed",
    "loop",
    "extended",
    "reaction",
    "tutorial",
    "review",
    "session",
    "rehearsal",
    "compilacion",
    "lofi",
    "8d",
    "boosted",
    "muffled",
    "demo",
    "tiktok",
];

// Words that announce a re-recording rather than the master: a fan's cover, a
// tribute, a remake. They are held apart from VERSIONS because the word only
// gives someone away when the uploader is not the artist.
const COVERS: [&str; 4] = ["cover", "version", "tribute", "remake"];

// Channels that publish karaoke and lyric videos. They restate the song, so
// their running time is the video's and not the track's, which makes them look
// like a perfect duration match.
const LYRIC_CHANNELS: [&str; 6] = ["lyric", "lyrics", "letra", "letras", "lirik", "paroles"];

// The words uploaders and YouTube bolt onto titles without changing which song
// they are.
const NOISE: [&str; 15] = [
    "official",
    "video",
    "audio",
    "lyric",
    "lyrics",
    "letra",
    "letras",
    "hd",
    "hq",
    "4k",
    "remaster",
    "remastered",
    "visualizer",
    "explicit",
    "clean",
];

// The English and Spanish filler words. The same list filters the song and the
// candidate, so being generous with it is safe: a word dropped on both sides
// cannot cause a false mismatch.
const STOPWORDS: [&str; 40] = [
    "the", "a", "an", "of", "in", "on", "to", "and", "or", "for", "at", "is", "el", "la", "los",
    "las", "un", "una", "unos", "unas", "al", "de", "del", "con", "para", "por", "sin", "sobre",
    "entre", "desde", "hasta", "y", "en", "lo", "que", "se", "su", "sus", "le", "les",
];

/// Every video that can stand for the song, best match first. Both the
/// album-first and the artist-first query are searched and merged: an album
/// whose name matches another track buries the song it belongs to, and the
/// artist-first query still surfaces its official upload, which ranking then
/// places first.
pub async fn candidates(youtube: &Youtube, song: &Song) -> Result<Vec<Video>, String> {
    let mut found = Vec::new();
    let mut failure = None;

    for query in queries(song) {
        match youtube.search(&query).await {
            Ok(videos) => found.extend(videos),
            Err(error) => {
                failure.get_or_insert(error);
            }
        }
    }

    if found.is_empty() {
        return match failure {
            Some(error) => Err(error),
            None => Ok(found),
        };
    }

    Ok(rank(unique(found), song))
}

/// Orders the results by how well each video matches the song. It only
/// reorders: the caller still takes the first candidate that plays, so a poor
/// match stays a fallback instead of becoming a failure.
fn rank(videos: Vec<Video>, song: &Song) -> Vec<Video> {
    // The penalty is computed once per video: inside the comparator it would
    // be computed again on every comparison, and each one tokenizes the title
    // and builds a set of its words.
    let mut ranked: Vec<(f64, Video)> = videos
        .into_iter()
        .map(|video| (penalty(&video, song), video))
        .collect();
    ranked.sort_by(|left, right| left.0.total_cmp(&right.0));
    ranked.into_iter().map(|(_, video)| video).collect()
}

fn unique(videos: Vec<Video>) -> Vec<Video> {
    let mut seen = HashSet::new();
    videos
        .into_iter()
        .filter(|video| !video.id.is_empty() && seen.insert(video.id.clone()))
        .collect()
}

fn penalty(video: &Video, song: &Song) -> f64 {
    let gap = duration_gap(video.duration, song.duration);
    let tier = upload(video, song);
    missing(&video.title, song)
        + (gap * DURATION_WEIGHT).min(DURATION_CAP)
        + version(&video.title)
        + cover(&video.title, tier)
        + credit(video, song, tier)
        + trust(tier, gap)
}

/// Scales an uploader tier by how close the video's length is to the track's, so
/// the tier only ever breaks ties and never overrides a length that says it is a
/// different recording. A gap the two ends cannot measure leaves the tier whole,
/// because then length has said nothing either way.
fn trust(tier: f64, gap: f64) -> f64 {
    if tier >= 0.0 || gap <= TIER_FULL {
        return tier;
    }

    if gap >= TIER_GONE {
        return 0.0;
    }

    tier * (TIER_GONE - gap) / (TIER_GONE - TIER_FULL)
}

/// Penalises a title that announces a live cut, a reworked take or a degraded
/// re-upload. The count is capped so one word cannot bury a video that otherwise
/// matches the track exactly.
fn version(title: &str) -> f64 {
    let hits = tokens(title)
        .iter()
        .filter(|token| VERSIONS.contains(&token.as_str()))
        .count();
    (hits as f64 * KEYWORD_PENALTY).min(KEYWORD_CAP)
}

/// Penalises a title that announces a re-recording, but only when the uploader
/// is neither the artist nor their Topic channel: the artist's own "(Cover
/// Audio)" single is the master and must not be demoted for naming itself
/// honestly.
fn cover(title: &str, tier: f64) -> f64 {
    if tier < 0.0 {
        return 0.0;
    }

    let hits = tokens(title)
        .iter()
        .filter(|token| COVERS.contains(&token.as_str()))
        .count();
    (hits as f64 * KEYWORD_PENALTY).min(KEYWORD_CAP)
}

/// Rewards an upload whose title names the song's artist, the "Artist - Song"
/// convention official and re-uploaded masters use. It only fires when the
/// channel was not recognised, so it never stacks on the uploader tier, and it
/// is what separates the original from a different artist's take on the same
/// title when both match the length.
fn credit(video: &Video, song: &Song, tier: f64) -> f64 {
    if tier < 0.0 {
        return 0.0;
    }

    let have: HashSet<String> = tokens(&video.title).into_iter().collect();
    for artist in &song.artists {
        let want = tokens(artist);
        if !want.is_empty() && want.iter().all(|token| have.contains(token)) {
            return CREDIT_BONUS;
        }
    }

    0.0
}

/// Scores who uploaded the video. The official artist badge, when YouTube sends
/// it, settles the channel before anything else: it is the artist's own upload
/// even when the title says "lyrics", and without that a same-title upload by
/// another artist outranks the master. When there is no badge, the lyric words
/// are looked for in the raw title, because a lyric channel restates the song
/// and its running time is the video's, not the track's.
fn upload(video: &Video, song: &Song) -> f64 {
    let channel = video.author.trim().to_lowercase();
    let title = video.title.to_lowercase();

    if video.official {
        return CHANNEL_BONUS;
    }

    if LYRIC_CHANNELS
        .iter()
        .any(|word| channel.contains(word) || title.contains(word))
    {
        return LYRIC_PENALTY;
    }

    if channel.is_empty() {
        return 0.0;
    }

    if TOPIC_SUFFIXES
        .iter()
        .any(|suffix| channel.ends_with(suffix))
    {
        return TOPIC_BONUS;
    }

    for artist in &song.artists {
        if artist.is_empty() {
            continue;
        }

        // A name that merely starts with the artist's is a collaboration channel
        // or a fan's, and must not inherit the artist's trust.
        let name = artist.to_lowercase();
        if channel == name {
            return CHANNEL_BONUS;
        }

        if OFFICIAL_SUFFIXES.iter().any(|suffix| {
            channel == format!("{name}{suffix}") || channel.starts_with(&format!("{name} {suffix}"))
        }) {
            return CHANNEL_BONUS;
        }
    }

    0.0
}

/// Penalises every word of the song's title that the candidate's title never
/// mentions. This is what demotes a different song by the same artist, which the
/// duration check cannot see.
fn missing(title: &str, song: &Song) -> f64 {
    let want: HashSet<String> = tokens(&unbracket(&song.name))
        .into_iter()
        .filter(|token| !is_year(token))
        .collect();
    if want.is_empty() {
        return 0.0;
    }

    let have: HashSet<String> = tokens(title).into_iter().collect();
    let count = want.iter().filter(|token| !have.contains(*token)).count();
    count as f64 * TOKEN_PENALTY
}

/// The relative difference between the video's length and the track's, which
/// demotes live cuts, extended loops and edited highlights.
fn duration_gap(candidate: u32, song: u32) -> f64 {
    if candidate == 0 || song == 0 {
        return 0.0;
    }

    (candidate as f64 - song as f64).abs() / song as f64
}

/// A bare four-digit year. Spotify hangs the remaster year onto the title and
/// almost no upload repeats it, so demanding it would bury the right video one
/// whole song's penalty down the list.
fn is_year(token: &str) -> bool {
    token.len() == 4 && token.parse::<u16>().is_ok()
}

/// Drops the tails Spotify hangs off a title, as in "Song (Remastered 2011)" or
/// "Song [feat. X]", which say nothing about which song it is.
fn unbracket(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut depth = 0usize;
    for value in text.chars() {
        match value {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(value),
            _ => {}
        }
    }

    out
}

fn tokens(text: &str) -> Vec<String> {
    text.to_lowercase()
        .replace("á", "a")
        .replace("é", "e")
        .replace("í", "i")
        .replace("ó", "o")
        .replace("ú", "u")
        .replace("ü", "u")
        .replace("ñ", "n")
        .replace("lo-fi", "lofi")
        .replace("lo fi", "lofi")
        .split(|value: char| !value.is_alphanumeric())
        .filter(|token| !token.is_empty() && !NOISE.contains(token) && !STOPWORDS.contains(token))
        .map(str::to_owned)
        .collect()
}

fn queries(song: &Song) -> Vec<String> {
    let name = song.name.trim();
    if name.is_empty() {
        return Vec::new();
    }

    let artists = song
        .artists
        .iter()
        .map(|artist| artist.trim())
        .filter(|artist| !artist.is_empty())
        .collect::<Vec<_>>()
        .join(", ");

    let mut album = if artists.is_empty() {
        name.to_owned()
    } else {
        format!("{name} {artists}")
    };
    if !song.album.trim().is_empty() {
        album = format!("{album} {}", song.album.trim());
    }

    let artist = if artists.is_empty() {
        name.to_owned()
    } else {
        format!("{artists} {name}")
    };

    if artist == album {
        return vec![album];
    }

    vec![album, artist]
}

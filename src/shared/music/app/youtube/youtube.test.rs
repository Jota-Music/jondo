use std::collections::HashSet;

use serde_json::{Value, json};

#[test]
fn cookies_are_read_from_a_jar_and_a_header() {
    let jar = "# Netscape HTTP Cookie File\n.youtube.com\tTRUE\t/\tTRUE\t0\tSAPISID\tabc\n.youtube.com\tTRUE\t/\tTRUE\t0\t__Secure-3PSID\tdef\n";
    let parsed = parse_cookies(jar).unwrap();
    assert_eq!(parsed, "SAPISID=abc; __Secure-3PSID=def");
    assert!(full(&parsed));

    let header = "Cookie: SAPISID=abc; VISITOR_INFO1_LIVE=zzz";
    assert_eq!(
        parse_cookies(header).unwrap(),
        "SAPISID=abc; VISITOR_INFO1_LIVE=zzz"
    );
    assert!(!full(&parse_cookies(header).unwrap()));
    assert!(parse_cookies("VISITOR_INFO1_LIVE=zzz").is_none());
}

#[test]
fn the_authorization_header_carries_the_origin() {
    let hash = sapisid_hash("SAPISID=abc", "https://www.youtube.com").unwrap();
    assert!(hash.starts_with("SAPISIDHASH "));
    assert_eq!(hash.split('_').count(), 2);
    assert_eq!(hash.split('_').nth(1).unwrap().len(), 40);
}

#[test]
fn the_same_session_ignores_the_rotated_cookies() {
    assert!(same(
        "SAPISID=abc; __Secure-1PSIDTS=one",
        "SAPISID=abc; __Secure-1PSIDTS=two"
    ));
    assert!(!same("SAPISID=abc", "SAPISID=def"));
    assert!(!same("VISITOR_INFO1_LIVE=zzz", "SAPISID=abc"));
}

#[test]
fn the_library_reads_lockup_cards_and_skips_videos() {
    let value = json!({
        "contents": {"sectionListRenderer": {"contents": [
            {"itemSectionRenderer": {"contents": [
                {"gridRenderer": {"items": [
                    {"lockupViewModel": {
                        "contentId": "PLabc",
                        "contentType": "LOCKUP_CONTENT_TYPE_PLAYLIST",
                        "contentImage": {"collectionThumbnailViewModel": {"primaryThumbnail": {
                            "thumbnailViewModel": {"image": {"sources": [
                                {"url": "https://i.ytimg.com/vi/abc/hqdefault.jpg"}
                            ]}}
                        }}},
                        "metadata": {"lockupMetadataViewModel": {"title": {"content": "Mine"}}}
                    }},
                    {"lockupViewModel": {
                        "contentId": "abc12345678",
                        "contentType": "LOCKUP_CONTENT_TYPE_VIDEO",
                        "metadata": {"lockupMetadataViewModel": {"title": {"content": "A video"}}}
                    }}
                ]}}
            ]}}
        ]}}
    });

    let playlists = extract_library(&value);
    assert_eq!(playlists.len(), 1);
    assert_eq!(playlists[0].id, "youtube:playlist:PLabc");
    assert_eq!(playlists[0].name, "Mine");
    assert_eq!(
        playlists[0].cover.as_deref(),
        Some("https://i.ytimg.com/vi/abc/hqdefault.jpg")
    );
}

#[test]
fn an_oembed_author_url_names_its_channel() {
    assert_eq!(
        channel_ref("https://www.youtube.com/channel/UCq19-LqvG35A-30oyAiPiqA").as_deref(),
        Some("UCq19-LqvG35A-30oyAiPiqA")
    );
    assert_eq!(
        channel_ref("https://www.youtube.com/@LofiGirl").as_deref(),
        Some("@LofiGirl")
    );
    assert_eq!(channel_ref("https://www.youtube.com/c/LofiGirl"), None);
}

#[test]
fn a_playlist_video_keeps_the_channel_it_links_to() {
    let renderer = json!({
        "videoId": "abc12345678",
        "title": {"runs": [{"text": "A video"}]},
        "shortBylineText": {"runs": [{
            "text": "Lofi Girl",
            "navigationEndpoint": {"browseEndpoint": {"browseId": "UClofi"}}
        }]},
        "lengthText": {"simpleText": "1:00"}
    });

    let mut seen = HashSet::new();
    let song = playlist_song(&renderer, "", "My mix", &mut seen).unwrap();
    assert_eq!(song.artists, vec!["Lofi Girl".to_owned()]);
    assert_eq!(song.artist_ids, vec!["UClofi".to_owned()]);
    assert_eq!(song.album, "My mix");
}

#[test]
fn a_lockup_video_keeps_the_channel_buried_in_its_metadata() {
    let view = json!({
        "contentId": "abc12345678",
        "contentType": "LOCKUP_CONTENT_TYPE_VIDEO",
        "metadata": {"lockupMetadataViewModel": {
            "title": {"content": "A video"},
            "metadata": {"contentMetadataViewModel": {"metadataRows": [
                {"metadataParts": [{"text": {
                    "content": "Lofi Girl",
                    "commandRuns": [{"onTap": {"innertubeCommand": {
                        "browseEndpoint": {
                            "browseId": "UCq19-LqvG35A-30oyAiPiqA",
                            "canonicalBaseUrl": "/@LofiGirl"
                        }
                    }}}]
                }}]}
            ]}}
        }}
    });

    let mut seen = HashSet::new();
    let song = lockup_song(&view, "", "My mix", &mut seen).unwrap();
    assert_eq!(song.album, "My mix");
    assert_eq!(song.artist_ids, vec!["UCq19-LqvG35A-30oyAiPiqA".to_owned()]);
}

#[test]
fn a_channel_page_video_links_back_to_its_channel() {
    let value = json!({
        "videoRenderer": {
            "videoId": "abc12345678",
            "title": {"runs": [{"text": "A video"}]}
        }
    });

    let mut videos = Vec::new();
    let mut seen = HashSet::new();
    collect_videos(&value, "Lofi Girl", "UClofi", &mut videos, &mut seen);
    assert_eq!(videos.len(), 1);
    assert_eq!(videos[0].song().artist_ids, vec!["UClofi".to_owned()]);
}

#[test]
fn a_search_video_keeps_the_channel_it_links_to() {
    let value = json!({
        "contents": {"sectionListRenderer": {"contents": [
            {"itemSectionRenderer": {"contents": [
                {"videoRenderer": {
                    "videoId": "abc12345678",
                    "title": {"runs": [{"text": "A video"}]},
                    "longBylineText": {"runs": [{
                        "text": "Lofi Girl",
                        "navigationEndpoint": {"browseEndpoint": {"browseId": "UClofi"}}
                    }]},
                    "lengthText": {"simpleText": "1:00"}
                }}
            ]}}
        ]}}
    });

    let videos = extract_videos(&value);
    assert_eq!(videos.len(), 1);
    assert_eq!(videos[0].channel.as_deref(), Some("UClofi"));
    assert_eq!(videos[0].song().artist_ids, vec!["UClofi".to_owned()]);
}

#[test]
fn audio_prefers_the_original_track_over_auto_dubs() {
    let value = json!({
        "streamingData": {"adaptiveFormats": [
            {"itag": 140, "mimeType": "audio/mp4; codecs=\"mp4a.40.2\"", "url": "https://x/dub-ar",
             "audioTrack": {"id": "ar.10", "audioIsDefault": false, "isAutoDubbed": true}},
            {"itag": 141, "mimeType": "audio/mp4; codecs=\"mp4a.40.2\"", "url": "https://x/dub-en",
             "audioTrack": {"id": "en-US.10", "audioIsDefault": false, "isAutoDubbed": true}},
            {"itag": 140, "mimeType": "audio/mp4; codecs=\"mp4a.40.2\"", "url": "https://x/original",
             "audioTrack": {"id": "es-US.4", "audioIsDefault": true}}
        ]}
    });

    assert_eq!(pick_audio(&value).unwrap().0, "https://x/original");
}

#[test]
fn channel_avatars_survive_a_protocol_relative_url() {
    let value = json!({
        "contents": {"twoColumnSearchResultsRenderer": {"primaryContents": {
            "sectionListRenderer": {"contents": [{"itemSectionRenderer": {"contents": [
                {"channelRenderer": {
                    "channelId": "UCabc",
                    "title": {"simpleText": "Lofi"},
                    "thumbnail": {"thumbnails": [
                        {"url": "//yt3.ggpht.com/abc=s88", "width": 88, "height": 88},
                        {"url": "//yt3.ggpht.com/abc=s176", "width": 176, "height": 176}
                    ]}
                }},
                {"channelRenderer": {
                    "channelId": "UCdef",
                    "title": {"simpleText": "Jazz"},
                    "thumbnail": {"thumbnails": [
                        {"url": "https://yt3.ggpht.com/def=s176", "width": 176, "height": 176}
                    ]}
                }}
            ]}}]}
        }}}
    });

    let channels = extract_channels(&value);
    assert_eq!(
        channels[0].avatar.as_deref(),
        Some("https://yt3.ggpht.com/abc=s176")
    );
    assert_eq!(
        channels[1].avatar.as_deref(),
        Some("https://yt3.ggpht.com/def=s176")
    );
}

#[test]
fn a_playlist_search_result_carries_its_video_count() {
    let value = json!({
        "playlistId": "PLabc",
        "title": {"runs": [{"text": "Mine"}]},
        "shortBylineText": {"runs": [{"text": "Owner"}]},
        "videoCountShortText": {"runs": [{"text": "15"}]},
        "thumbnail": {"thumbnails": [{"url": "https://x/cover.jpg"}]}
    });

    let playlist = rendered(&value).unwrap();
    assert_eq!(playlist.track_count, 15);
    assert_eq!(playlist.owner.as_deref(), Some("Owner"));
}

#[test]
fn a_library_lockup_carries_its_owner() {
    let value = json!({
        "contentType": "LOCKUP_CONTENT_TYPE_PLAYLIST",
        "contentId": "PLabc",
        "metadata": {"lockupMetadataViewModel": {
            "title": {"content": "Mine"},
            "metadata": {"contentMetadataViewModel": {"metadataRows": [
                {"metadataParts": [{"text": {"content": "Javier"}}]}
            ]}}
        }}
    });

    let playlist = lockup(&value).unwrap();
    assert_eq!(playlist.subtitle.as_deref(), Some("Javier"));
    assert!(playlist.owner.is_none());
}

#[test]
fn the_library_page_hands_its_initial_data() {
    let html =
        r#"<script>var ytInitialData = {"contents":{"a":1}};</script><script>var x = 2;</script>"#;

    let data = initial_data(html).unwrap();
    assert_eq!(data.pointer("/contents/a").and_then(Value::as_u64), Some(1));
}

use super::{cookies::*, extract_playlists::*, extract_videos::*};
use crate::shared::kernel::app::mutex;

/// Which player client answers fastest. Ignored: it hits YouTube. The app asks
/// one client per attempt, so this is the whole startup cost per track.
#[test]
#[ignore = "hits youtube.com"]
fn the_player_clients_are_measured_one_by_one() {
    use crate::shared::youtube::Youtube;
    use crate::shared::youtube::client::{CLIENTS, WEB};

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio");
    let youtube = Youtube::new();
    let video = std::env::var("JONDO_BENCH_VIDEO").unwrap_or_else(|_| "dQw4w9WgXcQ".to_owned());

    let ask = |client| {
        let payload = json!({
            "videoId": video,
            "context": {"client": youtube.context(client)},
            "contentCheckOk": true,
            "racyCheckOk": true,
        });

        let started = std::time::Instant::now();
        let status = runtime
            .block_on(youtube.post(client, super::PLAYER_URL, payload, false))
            .map(|value| {
                value
                    .pointer("/playabilityStatus/status")
                    .and_then(Value::as_str)
                    .unwrap_or("?")
                    .to_owned()
            })
            .unwrap_or_else(|error| error);

        (client.name, started.elapsed(), status)
    };

    let clients = CLIENTS.iter().copied().chain([WEB]).collect::<Vec<_>>();

    // Warm the pool and the visitor first: the first call of a process pays
    // the homepage scrape as well, and that is not a client's fault.
    ask(clients[0]);

    for round in 0..3 {
        for client in clients.iter().copied() {
            let (name, took, status) = ask(client);
            println!(
                "round {round} {name:>12} {:>7.0} ms  {status}",
                took.as_secs_f64() * 1000.0
            );
        }
    }
}

/// A resolved URL is reused, and one past its window is not: a stale URL would
/// surface as a 403 in the middle of a song.
#[test]
fn a_stream_is_kept_until_its_window_closes() {
    use crate::shared::music::domain::Stream;
    use crate::shared::youtube::Youtube;
    use std::time::Instant;

    let youtube = Youtube::new();
    let stream = Stream {
        url: "https://example/audio".to_owned(),
        agent: "test".to_owned(),
        duration: 3.0,
    };

    assert!(youtube.cached("abc").is_none());

    let now = Instant::now();
    mutex::lock(&youtube.streams).insert("abc".to_owned(), (stream.clone(), now));
    assert_eq!(youtube.cached("abc").unwrap().url, stream.url);

    let stale = now - super::stream::CACHED - std::time::Duration::from_secs(1);
    mutex::lock(&youtube.streams).insert("old".to_owned(), (stream, stale));
    assert!(youtube.cached("old").is_none());
}

/// How long YouTube takes to turn a title and an artist into a video. Ignored:
/// it hits YouTube. This is the one-time cost of the first play of a song that
/// has no linked video yet.
#[test]
#[ignore = "hits youtube.com"]
fn the_search_is_measured() {
    use crate::shared::youtube::Youtube;
    use std::time::Instant;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio");
    let youtube = Youtube::new();
    let query = "bohemian rhapsody queen";

    for round in 0..3 {
        let started = Instant::now();
        let hits = runtime.block_on(youtube.search(query));
        println!(
            "search round {round} {:>6.0} ms  {} hits",
            started.elapsed().as_secs_f64() * 1000.0,
            hits.map(|videos| videos.len()).unwrap_or(0)
        );
    }
}

use crate::auth::BrowserToken;
use crate::common::{AlbumID, ArtistChannelID, PlaylistID, YoutubeID};
use crate::parse::SearchResults;
use crate::process_json;
use crate::query::search::{
    AlbumsFilter, ArtistsFilter, CommunityPlaylistsFilter, EpisodesFilter, FeaturedPlaylistsFilter,
    PlaylistsFilter, PodcastsFilter, ProfilesFilter, SearchQuery, SongsFilter, VideosFilter,
};
use pretty_assertions::assert_eq;
use serde_json::{Value, json};
use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

#[tokio::test]
async fn test_search_basic_top_result_no_type() {
    // Case where topmost result doesn't contain a type.
    parse_test!(
        "./test_json/search_basic_top_result_no_type_20240720.json",
        "./test_json/search_basic_top_result_no_type_20240720_output.txt",
        SearchQuery::new(""),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_basic_radio() {
    // Case where topmost result is a special 'radio' playlist. Doesn't contain a
    // type and only has a single subtitle. Seems to show up when searching for
    // genres like classical and metal.
    parse_test!(
        "./test_json/search_basic_radio_20240830.json",
        "./test_json/search_basic_radio_20240830_output.txt",
        SearchQuery::new(""),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_basic_top_result_card() {
    // Case where there is only a 'card' top result, with no children.
    parse_test!(
        "./test_json/search_basic_top_result_card_20240721.json",
        "./test_json/search_basic_top_result_card_20240721_output.txt",
        SearchQuery::new(""),
        BrowserToken
    );
}
#[tokio::test]
async fn test_basic_search_no_results_suggestions() {
    // Case where there are no results, but there are 'Did You Mean' suggestions.
    parse_test_value!(
        "./test_json/search_basic_no_results_suggestions_20240104.json",
        SearchResults::default(),
        SearchQuery::new(""),
        BrowserToken
    );
}

#[tokio::test]
async fn test_search_basic_no_results() {
    // Case where there are no results, and there are not 'Did You Mean'
    // suggestions.
    parse_test!(
        "./test_json/search_basic_no_results_20240721.json",
        "./test_json/search_basic_no_results_20240721_output.txt",
        SearchQuery::new(""),
        BrowserToken
    );
}

#[tokio::test]
async fn test_search_artists_empty() {
    let source_path = Path::new("./test_json/search_artists_no_results_20231226.json");
    let source = tokio::fs::read_to_string(source_path)
        .await
        .expect("Expect file read to pass during tests");
    // Blank query has no bearing on function
    let query = SearchQuery::new("").with_filter(ArtistsFilter);
    let output = process_json::<_, BrowserToken>(source, query).unwrap();
    assert_eq!(output, Vec::new());
}
#[tokio::test]
// Test results appear for the correct categories.
async fn test_basic_search_has_simple_top_result() {
    let source_path = Path::new("./test_json/search_basic_top_result_20231228.json");
    let source = tokio::fs::read_to_string(source_path)
        .await
        .expect("Expect file read to pass during tests");
    // Blank query has no bearing on function
    let query = SearchQuery::new("");
    let output = process_json::<_, BrowserToken>(source, query).unwrap();
    assert!(!output.top_results.is_empty());
}
#[tokio::test]
// Test results appear for the correct categories.
async fn test_basic_search_has_card_top_result() {
    let source_path = Path::new("./test_json/search_highlighted_top_result_20240107.json");
    let source = tokio::fs::read_to_string(source_path)
        .await
        .expect("Expect file read to pass during tests");
    // Blank query has no bearing on function
    let query = SearchQuery::new("");
    let output = process_json::<_, BrowserToken>(source, query).unwrap();
    assert!(!output.top_results.is_empty());
}
#[tokio::test]
// Test results appear for the correct categories.
async fn test_basic_search_no_top_results_has_results() {
    let source_path = Path::new("./test_json/search_basic_no_top_result_20231228.json");
    let source = tokio::fs::read_to_string(source_path)
        .await
        .expect("Expect file read to pass during tests");
    // Blank query has no bearing on function
    let query = SearchQuery::new("");
    let output = process_json::<_, BrowserToken>(source, query).unwrap();
    assert!(!output.songs.is_empty());
    assert!(!output.featured_playlists.is_empty());
    assert!(!output.videos.is_empty());
    assert!(!output.community_playlists.is_empty());
    assert!(!output.episodes.is_empty());
    assert!(!output.artists.is_empty());
    assert!(!output.podcasts.is_empty());
    assert!(!output.profiles.is_empty());
    assert!(output.top_results.is_empty());
}

#[tokio::test]
async fn test_basic_search_highlighted_top_result() {
    parse_test!(
        "./test_json/search_highlighted_top_result_20240107.json",
        "./test_json/search_highlighted_top_result_20240107_output.txt",
        SearchQuery::new(""),
        BrowserToken
    );
}
#[tokio::test]
async fn test_basic_search_with_vodcasts_type_not_specified() {
    parse_test!(
        "./test_json/search_basic_with_vodcasts_type_not_specified_20240612.json",
        "./test_json/search_basic_with_vodcasts_type_not_specified_20240612_output.txt",
        SearchQuery::new(""),
        BrowserToken
    );
}
#[tokio::test]
async fn test_basic_search_with_vodcasts_type_specified() {
    parse_test!(
        "./test_json/search_basic_with_vodcasts_type_specified_20240612.json",
        "./test_json/search_basic_with_vodcasts_type_specified_20240612_output.txt",
        SearchQuery::new(""),
        BrowserToken
    );
}
#[tokio::test]
async fn test_basic_search_with_about_message() {
    parse_test!(
        "./test_json/search_basic_with_about_message_20240809.json",
        "./test_json/search_basic_with_about_message_20240809_output.txt",
        SearchQuery::new(""),
        BrowserToken
    );
}
#[tokio::test]
async fn test_basic_search_with_podcast_community_playlists() {
    parse_test!(
        "./test_json/search_basic_with_podcast_community_playlists_20250605.json",
        "./test_json/search_basic_with_podcast_community_playlists_20250605_output.txt",
        SearchQuery::new(""),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_artists() {
    parse_with_matching_continuation_test!(
        "./test_json/search_artists_20231226.json",
        "./test_json/search_artists_continuation_20231226.json",
        "./test_json/search_artists_20231226_output.txt",
        SearchQuery::new("").with_filter(ArtistsFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_artists_with_about_message() {
    parse_test!(
        "./test_json/search_artists_with_about_message_20240824.json",
        "./test_json/search_artists_with_about_message_20240824_output.txt",
        SearchQuery::new("").with_filter(ArtistsFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_albums() {
    parse_with_matching_continuation_test!(
        "./test_json/search_albums_20231226.json",
        "./test_json/search_albums_continuation_20231226.json",
        "./test_json/search_albums_20231226_output.txt",
        SearchQuery::new("").with_filter(AlbumsFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_songs() {
    parse_with_matching_continuation_test!(
        "./test_json/search_songs_20231226.json",
        "./test_json/search_songs_continuation_20231226.json",
        "./test_json/search_songs_20231226_output.txt",
        SearchQuery::new("").with_filter(SongsFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_videos() {
    parse_test!(
        "./test_json/search_videos_20231226.json",
        "./test_json/search_videos_20231226_output.txt",
        SearchQuery::new("").with_filter(VideosFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_videos_2024() {
    // Vodcasts were added for this version
    parse_with_matching_continuation_test!(
        "./test_json/search_videos_20240612.json",
        "./test_json/search_videos_continuation_20240612.json",
        "./test_json/search_videos_20240612_output.txt",
        SearchQuery::new("").with_filter(VideosFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_playlists() {
    parse_with_matching_continuation_test!(
        "./test_json/search_playlists_20231228.json",
        "./test_json/search_playlists_continuation_20231228.json",
        "./test_json/search_playlists_20231228_output.txt",
        SearchQuery::new("").with_filter(PlaylistsFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_featured_playlists() {
    parse_with_matching_continuation_test!(
        "./test_json/search_featured_playlists_20231226.json",
        "./test_json/search_featured_playlists_continuation_20231226.json",
        "./test_json/search_featured_playlists_20231226_output.txt",
        SearchQuery::new("").with_filter(FeaturedPlaylistsFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_community_playlists() {
    parse_with_matching_continuation_test!(
        "./test_json/search_community_playlists_20231226.json",
        "./test_json/search_community_playlists_continuation_20231226.json",
        "./test_json/search_community_playlists_20231226_output.txt",
        SearchQuery::new("").with_filter(CommunityPlaylistsFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_episodes() {
    parse_with_matching_continuation_test!(
        "./test_json/search_episodes_20231226.json",
        "./test_json/search_episodes_continuation_20231226.json",
        "./test_json/search_episodes_20231226_output.txt",
        SearchQuery::new("").with_filter(EpisodesFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_podcasts() {
    parse_with_matching_continuation_test!(
        "./test_json/search_podcasts_20231226.json",
        "./test_json/search_podcasts_continuation_20231226.json",
        "./test_json/search_podcasts_20231226_output.txt",
        SearchQuery::new("").with_filter(PodcastsFilter),
        BrowserToken
    );
}
#[tokio::test]
async fn test_search_profiles() {
    parse_with_matching_continuation_test!(
        "./test_json/search_profiles_20231226.json",
        "./test_json/search_profiles_continuation_20231226.json",
        "./test_json/search_profiles_20231226_output.txt",
        SearchQuery::new("").with_filter(ProfilesFilter),
        BrowserToken
    );
}

#[tokio::test]
async fn test_basic_search_artist_browse_id_falls_back_to_title_run() {
    let source = json!({
        "contents": {
            "tabbedSearchResultsRenderer": {
                "tabs": [{
                    "tabRenderer": {
                        "content": {
                            "sectionListRenderer": {
                                "contents": [{
                                    "musicShelfRenderer": {
                                        "title": { "runs": [{ "text": "Artists" }] },
                                        "contents": [{
                                            "musicResponsiveListItemRenderer": {
                                                "flexColumns": [
                                                    {
                                                        "musicResponsiveListItemFlexColumnRenderer": {
                                                            "text": {
                                                                "runs": [{
                                                                    "text": "Hoàng Dũng",
                                                                    "navigationEndpoint": {
                                                                        "browseEndpoint": {
                                                                            "browseId": "UC1ZSSNeXsPb5XKNv-WnBcdg"
                                                                        }
                                                                    }
                                                                }]
                                                            }
                                                        }
                                                    },
                                                    {
                                                        "musicResponsiveListItemFlexColumnRenderer": {
                                                            "text": {
                                                                "runs": [
                                                                    { "text": "Artist" },
                                                                    { "text": " • " },
                                                                    { "text": "4.63M monthly audience" }
                                                                ]
                                                            }
                                                        }
                                                    }
                                                ],
                                                "thumbnail": {
                                                    "musicThumbnailRenderer": {
                                                        "thumbnail": { "thumbnails": [] }
                                                    }
                                                }
                                            }
                                        }]
                                    }
                                }]
                            }
                        }
                    }
                }]
            }
        }
    })
    .to_string();

    let output = process_json::<_, BrowserToken>(source, SearchQuery::new("")).unwrap();

    assert_eq!(output.artists.len(), 1);
    assert_eq!(
        output.artists[0].browse_id,
        ArtistChannelID::from_raw("UC1ZSSNeXsPb5XKNv-WnBcdg")
    );
}

#[tokio::test]
async fn test_basic_search_album_browse_id_falls_back_to_title_run() {
    let source = json!({
        "contents": {
            "tabbedSearchResultsRenderer": {
                "tabs": [{
                    "tabRenderer": {
                        "content": {
                            "sectionListRenderer": {
                                "contents": [{
                                    "musicShelfRenderer": {
                                        "title": { "runs": [{ "text": "Albums" }] },
                                        "contents": [{
                                            "musicResponsiveListItemRenderer": {
                                                "flexColumns": [
                                                    {
                                                        "musicResponsiveListItemFlexColumnRenderer": {
                                                            "text": {
                                                                "runs": [{
                                                                    "text": "XOAY TRÒN",
                                                                    "navigationEndpoint": {
                                                                        "browseEndpoint": {
                                                                            "browseId": "MPREb_6SZWneqfbpW"
                                                                        }
                                                                    }
                                                                }]
                                                            }
                                                        }
                                                    },
                                                    {
                                                        "musicResponsiveListItemFlexColumnRenderer": {
                                                            "text": {
                                                                "runs": [
                                                                    { "text": "Album" },
                                                                    { "text": " • " },
                                                                    {
                                                                        "text": "Hoàng Dũng",
                                                                        "navigationEndpoint": {
                                                                            "browseEndpoint": {
                                                                                "browseId": "UC1ZSSNeXsPb5XKNv-WnBcdg"
                                                                            }
                                                                        }
                                                                    },
                                                                    { "text": " • " },
                                                                    { "text": "2025" }
                                                                ]
                                                            }
                                                        }
                                                    }
                                                ],
                                                "thumbnail": {
                                                    "musicThumbnailRenderer": {
                                                        "thumbnail": { "thumbnails": [] }
                                                    }
                                                }
                                            }
                                        }]
                                    }
                                }]
                            }
                        }
                    }
                }]
            }
        }
    })
    .to_string();

    let output = process_json::<_, BrowserToken>(source, SearchQuery::new("")).unwrap();

    assert_eq!(output.albums.len(), 1);
    assert_eq!(
        output.albums[0].album_id,
        AlbumID::from_raw("MPREb_6SZWneqfbpW")
    );
}

#[tokio::test]
async fn test_basic_search_featured_playlist_browse_id_falls_back_to_title_run() {
    let source = json!({
        "contents": {
            "tabbedSearchResultsRenderer": {
                "tabs": [{
                    "tabRenderer": {
                        "content": {
                            "sectionListRenderer": {
                                "contents": [{
                                    "musicShelfRenderer": {
                                        "title": { "runs": [{ "text": "Featured playlists" }] },
                                        "contents": [{
                                            "musicResponsiveListItemRenderer": {
                                                "flexColumns": [
                                                    {
                                                        "musicResponsiveListItemFlexColumnRenderer": {
                                                            "text": {
                                                                "runs": [{
                                                                    "text": "Yên Concert Setlist",
                                                                    "navigationEndpoint": {
                                                                        "browseEndpoint": {
                                                                            "browseId": "VLPLtestplaylist123"
                                                                        }
                                                                    }
                                                                }]
                                                            }
                                                        }
                                                    },
                                                    {
                                                        "musicResponsiveListItemFlexColumnRenderer": {
                                                            "text": {
                                                                "runs": [
                                                                    { "text": "Hoàng Dũng" },
                                                                    { "text": " • " },
                                                                    { "text": "12 songs" }
                                                                ]
                                                            }
                                                        }
                                                    }
                                                ],
                                                "thumbnail": {
                                                    "musicThumbnailRenderer": {
                                                        "thumbnail": { "thumbnails": [] }
                                                    }
                                                }
                                            }
                                        }]
                                    }
                                }]
                            }
                        }
                    }
                }]
            }
        }
    })
    .to_string();

    let output = process_json::<_, BrowserToken>(source, SearchQuery::new("")).unwrap();

    assert_eq!(output.featured_playlists.len(), 1);
    assert_eq!(
        output.featured_playlists[0].playlist_id,
        PlaylistID::from_raw("VLPLtestplaylist123")
    );
}

#[tokio::test]
async fn test_basic_search_song_metadata_uses_segment_types_not_fixed_positions() {
    let source = json!({
        "contents": {
            "tabbedSearchResultsRenderer": {
                "tabs": [{
                    "tabRenderer": {
                        "content": {
                            "sectionListRenderer": {
                                "contents": [{
                                    "musicShelfRenderer": {
                                        "title": { "runs": [{ "text": "Songs" }] },
                                        "contents": [{
                                            "musicResponsiveListItemRenderer": {
                                                "flexColumns": [
                                                    {
                                                        "musicResponsiveListItemFlexColumnRenderer": {
                                                            "text": { "runs": [{ "text": "Thói Quen" }] }
                                                        }
                                                    },
                                                    {
                                                        "musicResponsiveListItemFlexColumnRenderer": {
                                                            "text": {
                                                                "runs": [
                                                                    {
                                                                        "text": "Hoàng Dũng",
                                                                        "navigationEndpoint": {
                                                                            "browseEndpoint": {
                                                                                "browseId": "UC1ZSSNeXsPb5XKNv-WnBcdg"
                                                                            }
                                                                        }
                                                                    },
                                                                    { "text": " • " },
                                                                    {
                                                                        "text": "25",
                                                                        "navigationEndpoint": {
                                                                            "browseEndpoint": {
                                                                                "browseId": "MPREb_YHjDMBCbrSj"
                                                                            }
                                                                        }
                                                                    },
                                                                    { "text": " • " },
                                                                    { "text": "4:31" }
                                                                ]
                                                            }
                                                        }
                                                    },
                                                    {
                                                        "musicResponsiveListItemFlexColumnRenderer": {
                                                            "text": { "runs": [{ "text": "348K plays" }] }
                                                        }
                                                    }
                                                ],
                                                "playlistItemData": { "videoId": "2r8chZA-Tvo" },
                                                "thumbnail": {
                                                    "musicThumbnailRenderer": {
                                                        "thumbnail": { "thumbnails": [] }
                                                    }
                                                }
                                            }
                                        }]
                                    }
                                }]
                            }
                        }
                    }
                }]
            }
        }
    })
    .to_string();

    let output = process_json::<_, BrowserToken>(source, SearchQuery::new("")).unwrap();

    assert_eq!(output.songs.len(), 1);
    assert_eq!(output.songs[0].artist, "Hoàng Dũng");
    assert_eq!(output.songs[0].duration, "4:31");
    assert_eq!(
        output.songs[0]
            .album
            .as_ref()
            .map(|album| album.name.as_str()),
        Some("25")
    );
    assert_eq!(
        output.songs[0].album.as_ref().map(|album| album.id.clone()),
        Some(AlbumID::from_raw("MPREb_YHjDMBCbrSj"))
    );
}

#[tokio::test]
async fn test_basic_search_top_result_card_browse_id_falls_back_to_title_run() {
    let source = json!({
        "contents": {
            "tabbedSearchResultsRenderer": {
                "tabs": [{
                    "tabRenderer": {
                        "content": {
                            "sectionListRenderer": {
                                "contents": [{
                                    "musicCardShelfRenderer": {
                                        "title": {
                                            "runs": [{
                                                "text": "Hoàng Dũng",
                                                "navigationEndpoint": {
                                                    "browseEndpoint": {
                                                        "browseId": "UC1ZSSNeXsPb5XKNv-WnBcdg"
                                                    }
                                                }
                                            }]
                                        },
                                        "subtitle": {
                                            "runs": [
                                                { "text": "Artist" },
                                                { "text": " • " },
                                                { "text": "4.63M monthly audience" }
                                            ]
                                        },
                                        "thumbnail": {
                                            "musicThumbnailRenderer": {
                                                "thumbnail": { "thumbnails": [] }
                                            }
                                        }
                                    }
                                }]
                            }
                        }
                    }
                }]
            }
        }
    })
    .to_string();

    let output = process_json::<_, BrowserToken>(source, SearchQuery::new("")).unwrap();

    assert_eq!(output.top_results.len(), 1);
    assert_eq!(
        output.top_results[0].browse_id.as_deref(),
        Some("UC1ZSSNeXsPb5XKNv-WnBcdg")
    );
}

#[tokio::test]
async fn test_basic_search_top_result_card_song_duration_uses_segment_parsing() {
    let source = json!({
        "contents": {
            "tabbedSearchResultsRenderer": {
                "tabs": [{
                    "tabRenderer": {
                        "content": {
                            "sectionListRenderer": {
                                "contents": [{
                                    "musicCardShelfRenderer": {
                                        "title": {
                                            "runs": [{
                                                "text": "Hoàng Dũng",
                                                "navigationEndpoint": {
                                                    "browseEndpoint": {
                                                        "browseId": "UC1ZSSNeXsPb5XKNv-WnBcdg"
                                                    }
                                                }
                                            }]
                                        },
                                        "subtitle": {
                                            "runs": [
                                                { "text": "Artist" },
                                                { "text": " • " },
                                                { "text": "4.63M monthly audience" }
                                            ]
                                        },
                                        "thumbnail": {
                                            "musicThumbnailRenderer": {
                                                "thumbnail": { "thumbnails": [] }
                                            }
                                        },
                                        "contents": [{
                                            "musicResponsiveListItemRenderer": {
                                                "flexColumns": [
                                                    {
                                                        "musicResponsiveListItemFlexColumnRenderer": {
                                                            "text": {
                                                                "runs": [{ "text": "Giữ Anh Cho Ngày Hôm Qua" }]
                                                            }
                                                        }
                                                    },
                                                    {
                                                        "musicResponsiveListItemFlexColumnRenderer": {
                                                            "text": {
                                                                "runs": [
                                                                    {
                                                                        "text": "Hoàng Dũng",
                                                                        "navigationEndpoint": {
                                                                            "browseEndpoint": {
                                                                                "browseId": "UC1ZSSNeXsPb5XKNv-WnBcdg"
                                                                            }
                                                                        }
                                                                    },
                                                                    { "text": ", " },
                                                                    {
                                                                        "text": "Rhymastic",
                                                                        "navigationEndpoint": {
                                                                            "browseEndpoint": {
                                                                                "browseId": "UCT1Gfuv8U23VlGl8xmDRUCg"
                                                                            }
                                                                        }
                                                                    },
                                                                    { "text": " & " },
                                                                    {
                                                                        "text": "Lelarec",
                                                                        "navigationEndpoint": {
                                                                            "browseEndpoint": {
                                                                                "browseId": "UC9cH5hT4lBGxQRKaWxyv_og"
                                                                            }
                                                                        }
                                                                    },
                                                                    { "text": " • " },
                                                                    { "text": "4:51" }
                                                                ]
                                                            }
                                                        }
                                                    }
                                                ],
                                                "playlistItemData": { "videoId": "mRXLNtzm9y0" },
                                                "thumbnail": {
                                                    "musicThumbnailRenderer": {
                                                        "thumbnail": { "thumbnails": [] }
                                                    }
                                                }
                                            }
                                        }]
                                    }
                                }]
                            }
                        }
                    }
                }]
            }
        }
    })
    .to_string();

    let output = process_json::<_, BrowserToken>(source, SearchQuery::new("")).unwrap();

    assert_eq!(output.top_results.len(), 2);
    assert_eq!(
        output.top_results[1].artist.as_deref(),
        Some("Hoàng Dũng, Rhymastic & Lelarec")
    );
    assert_eq!(output.top_results[1].duration.as_deref(), Some("4:51"));
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BenchTopResultsSample {
    primary_browse_id: Option<String>,
    child_rows: Vec<BenchTopResultRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BenchTopResultRow {
    title: String,
    artist: Option<String>,
    duration: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BenchRun {
    text: String,
    browse_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BenchSegment {
    text: String,
    browse_ids: Vec<String>,
}

fn load_tmp_search_json() -> (String, Value) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tmp/search.json");
    let source = std::fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
    let value = serde_json::from_str(&source)
        .unwrap_or_else(|err| panic!("failed to parse {}: {err}", path.display()));
    (source, value)
}

fn top_results_card<'a>(value: &'a Value) -> &'a Value {
    value
        .pointer(
            "/contents/tabbedSearchResultsRenderer/tabs/0/tabRenderer/content/sectionListRenderer/contents/0/musicCardShelfRenderer",
        )
        .expect("expected tmp/search.json to contain a top-results card")
}

fn is_duration_like(text: &str) -> bool {
    let parts: Vec<_> = text.trim().split(':').collect();
    (2..=3).contains(&parts.len())
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
}

fn run_text(run: &Value) -> Option<&str> {
    run.get("text").and_then(Value::as_str)
}

fn run_browse_id(run: &Value) -> Option<&str> {
    run.pointer("/navigationEndpoint/browseEndpoint/browseId")
        .and_then(Value::as_str)
}

fn extract_top_results_indexed(card: &Value) -> BenchTopResultsSample {
    let primary_browse_id = card
        .pointer("/title/runs/0/navigationEndpoint/browseEndpoint/browseId")
        .and_then(Value::as_str)
        .map(str::to_string);

    let child_rows = card
        .get("contents")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("musicResponsiveListItemRenderer"))
        .map(|row| {
            let title = row
                .pointer(
                    "/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/text",
                )
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();

            let runs = row
                .pointer("/flexColumns/1/musicResponsiveListItemFlexColumnRenderer/text/runs")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();

            let (artist, duration) = extract_indexed_metadata(&runs);

            BenchTopResultRow {
                title,
                artist,
                duration,
            }
        })
        .collect();

    BenchTopResultsSample {
        primary_browse_id,
        child_rows,
    }
}

fn extract_indexed_metadata(runs: &[Value]) -> (Option<String>, Option<String>) {
    if runs.is_empty() {
        return (None, None);
    }

    let duration = runs
        .last()
        .and_then(run_text)
        .filter(|text| is_duration_like(text))
        .map(str::to_string);

    let artist_end = if duration.is_some()
        && runs.len() >= 2
        && runs.get(runs.len() - 2).and_then(run_text) == Some(" • ")
    {
        runs.len() - 2
    } else {
        runs.len()
    };

    let artist = runs[..artist_end]
        .iter()
        .filter_map(run_text)
        .collect::<String>()
        .trim()
        .to_string();

    let artist = (!artist.is_empty()).then_some(artist);
    (artist, duration)
}

fn extract_top_results_normalized(card: &Value) -> BenchTopResultsSample {
    let title_runs = card
        .pointer("/title/runs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let primary_title_runs = normalize_runs(&title_runs);
    let primary_browse_id = primary_title_runs
        .iter()
        .find_map(|run| run.browse_id.clone());

    let child_rows = card
        .get("contents")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|item| item.get("musicResponsiveListItemRenderer"))
        .map(|row| {
            let title_runs = row
                .pointer("/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let metadata_runs = row
                .pointer("/flexColumns/1/musicResponsiveListItemFlexColumnRenderer/text/runs")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();

            let title = normalize_runs(&title_runs)
                .into_iter()
                .map(|run| run.text)
                .collect::<String>();
            let metadata_segments = split_bullet_segments(&normalize_runs(&metadata_runs));

            let duration = metadata_segments
                .last()
                .map(|segment| segment.text.trim())
                .filter(|text| is_duration_like(text))
                .map(str::to_string);

            let artist_segment_count = if duration.is_some() && !metadata_segments.is_empty() {
                metadata_segments.len().saturating_sub(1)
            } else {
                metadata_segments.len()
            };
            let artist = metadata_segments[..artist_segment_count]
                .iter()
                .map(|segment| segment.text.trim())
                .filter(|text| !text.is_empty())
                .collect::<Vec<_>>()
                .join(" • ");

            BenchTopResultRow {
                title,
                artist: (!artist.is_empty()).then_some(artist),
                duration,
            }
        })
        .collect();

    BenchTopResultsSample {
        primary_browse_id,
        child_rows,
    }
}

fn normalize_runs(runs: &[Value]) -> Vec<BenchRun> {
    runs.iter()
        .filter_map(|run| {
            Some(BenchRun {
                text: run_text(run)?.to_string(),
                browse_id: run_browse_id(run).map(str::to_string),
            })
        })
        .collect()
}

fn split_bullet_segments(runs: &[BenchRun]) -> Vec<BenchSegment> {
    let mut segments = Vec::new();
    let mut current = BenchSegment {
        text: String::new(),
        browse_ids: Vec::new(),
    };

    for run in runs {
        if run.text == " • " {
            if !current.text.trim().is_empty() {
                segments.push(current);
                current = BenchSegment {
                    text: String::new(),
                    browse_ids: Vec::new(),
                };
            }
            continue;
        }

        current.text.push_str(&run.text);
        if let Some(browse_id) = &run.browse_id {
            current.browse_ids.push(browse_id.clone());
        }
    }

    if !current.text.trim().is_empty() {
        segments.push(current);
    }

    segments
}

fn benchmark_iterations<T>(name: &str, iterations: usize, mut f: impl FnMut() -> T) -> f64 {
    for _ in 0..100 {
        black_box(f());
    }

    let started = Instant::now();
    for _ in 0..iterations {
        black_box(f());
    }
    let elapsed = started.elapsed();
    let nanos_per_iter = elapsed.as_nanos() as f64 / iterations as f64;

    eprintln!(
        "{name}: {:?} total over {iterations} iters ({nanos_per_iter:.1} ns/iter)",
        elapsed
    );

    nanos_per_iter
}

#[test]
#[ignore = "microbenchmark against local tmp/search.json"]
fn benchmark_tmp_search_top_results_index_vs_normalized() {
    let (source, value) = load_tmp_search_json();
    let card = top_results_card(&value);

    let indexed = extract_top_results_indexed(card);
    let normalized = extract_top_results_normalized(card);

    assert_eq!(
        indexed, normalized,
        "benchmark strategies diverged on tmp/search.json"
    );

    let indexed_ns = benchmark_iterations("indexed top-results extraction", 50_000, || {
        extract_top_results_indexed(black_box(card))
    });
    let normalized_ns = benchmark_iterations("normalized top-results extraction", 50_000, || {
        extract_top_results_normalized(black_box(card))
    });
    let end_to_end_ns =
        benchmark_iterations("current process_json end-to-end parse", 1_000, || {
            process_json::<_, BrowserToken>(black_box(source.clone()), SearchQuery::new(""))
                .unwrap()
        });

    eprintln!(
        "normalized/indexed ratio: {:.2}x | normalized/end-to-end: {:.2}%",
        normalized_ns / indexed_ns,
        (normalized_ns / end_to_end_ns) * 100.0
    );
}

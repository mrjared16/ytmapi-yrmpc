use super::{
    DISPLAY_POLICY, ParseFrom, ProcessedResult, flex_column_item_pointer, parse_flex_column_item,
};
use crate::common::{
    AlbumID, AlbumType, ArtistChannelID, ContinuationParams, EpisodeID, Explicit, PlaylistID,
    PodcastID, SearchSuggestion, SuggestionType, TextRun, Thumbnail, UserChannelID, VideoID,
    YoutubeID,
};
use crate::continuations::ParseFromContinuable;
use crate::nav_consts::{
    BADGE_LABEL, CONTINUATION_PARAMS, LIVE_BADGE_LABEL, MRLIR, MUSIC_CARD_SHELF, MUSIC_SHELF,
    MUSIC_SHELF_CONTINUATION, NAVIGATION_BROWSE, NAVIGATION_BROWSE_ID, ON_TAP_VIDEO_ID, PAGE_TYPE,
    PLAY_BUTTON, PLAYLIST_ITEM_VIDEO_ID, SECTION_LIST, SUBTITLE, SUBTITLE2, TAB_CONTENT,
    THUMBNAILS, TITLE_NAV_VIDEO_ID, TITLE_TEXT,
};
use crate::parse::{EpisodeDate, ParsedSongAlbum};
use crate::query::search::UnfilteredSearchType;
use crate::query::search::filteredsearch::{
    AlbumsFilter, ArtistsFilter, CommunityPlaylistsFilter, EpisodesFilter, FeaturedPlaylistsFilter,
    FilteredSearch, FilteredSearchType, PlaylistsFilter, PodcastsFilter, ProfilesFilter,
    SongsFilter, VideosFilter,
};
use crate::query::*;
use crate::youtube_enums::{PlaylistEndpointParams, YoutubeMusicPageType};
use crate::{Error, Result};
use const_format::concatcp;
use itertools::Itertools;
use json_crawler::{JsonCrawler, JsonCrawlerBorrowed, JsonCrawlerIterator, JsonCrawlerOwned};
use serde::de::IntoDeserializer;
use serde::{Deserialize, Serialize};

#[cfg(test)]
mod tests;

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SearchResults {
    pub top_results: Vec<TopResult>,
    pub artists: Vec<SearchResultArtist>,
    pub albums: Vec<SearchResultAlbum>,
    pub featured_playlists: Vec<SearchResultFeaturedPlaylist>,
    pub community_playlists: Vec<BasicSearchResultCommunityPlaylist>,
    pub songs: Vec<SearchResultSong>,
    pub videos: Vec<SearchResultVideo>,
    pub podcasts: Vec<SearchResultPodcast>,
    pub episodes: Vec<SearchResultEpisode>,
    pub profiles: Vec<SearchResultProfile>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
/// Each Top Result has it's own type.
pub enum TopResultType {
    Artist,
    Playlist,
    Song,
    Video,
    Station,
    Podcast,
    /// Unknown type - preserves the original string for debugging/logging
    Unknown(String),
    #[serde(untagged)]
    Album(AlbumType),
}

impl TopResultType {
    fn from_subtitle(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "song" => TopResultType::Song,
            "video" => TopResultType::Video,
            "artist" => TopResultType::Artist,
            "playlist" => TopResultType::Playlist,
            "station" => TopResultType::Station,
            "podcast" => TopResultType::Podcast,
            "album" => TopResultType::Album(AlbumType::Album),
            "single" => TopResultType::Album(AlbumType::Single),
            "ep" => TopResultType::Album(AlbumType::EP),
            _ => TopResultType::Unknown(s.to_string()),
        }
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// Helper enum for parsing different search result types.
enum SearchResultType {
    #[serde(alias = "Top result")]
    TopResult,
    Artists,
    Albums,
    #[serde(alias = "Featured playlists")]
    FeaturedPlaylists,
    #[serde(alias = "Community playlists")]
    CommunityPlaylists,
    Songs,
    Videos,
    Podcasts,
    Episodes,
    Profiles,
    #[serde(alias = "More results")]
    MoreResults,
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
/// Dynamically defined top result.
/// Some fields are optional as they are not defined for all result types.
// In future, may be possible to make this type safe.
// TODO: Add endpoint id.
pub struct TopResult {
    pub result_name: String,
    /// Both Videos and Songs can have this left out.
    pub result_type: Option<TopResultType>,
    pub thumbnails: Vec<Thumbnail>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration: Option<String>,
    pub year: Option<String>,
    pub subscribers: Option<String>,
    pub plays: Option<String>,
    /// Podcast publisher.
    pub publisher: Option<String>,
    /// Generic tagline that can appear on top results
    pub byline: Option<String>,
    pub browse_id: Option<String>,
    pub video_id: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
/// An artist search result.
pub struct SearchResultArtist {
    pub artist: String,
    /// An artist with no subscribers won't contain this field.
    pub subscribers: Option<String>,
    pub browse_id: ArtistChannelID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
/// A podcast search result.
pub struct SearchResultPodcast {
    pub title: String,
    pub publisher: String,
    pub podcast_id: PodcastID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
/// A podcast episode search result.
pub struct SearchResultEpisode {
    pub title: String,
    pub date: EpisodeDate,
    pub channel_name: String,
    pub episode_id: EpisodeID<'static>,
    // Potentially can include link to channel.
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// A video search result. May be a video or a video episode of a podcast.
pub enum SearchResultVideo {
    #[non_exhaustive]
    Video {
        title: String,
        /// Note: Either Youtube channel name, or artist name.
        // Potentially can include link to channel.
        channel_name: String,
        video_id: VideoID<'static>,
        views: String,
        length: String,
        thumbnails: Vec<Thumbnail>,
    },
    #[non_exhaustive]
    VideoEpisode {
        // Potentially asame as SearchResultEpisode
        title: String,
        date: EpisodeDate,
        channel_name: String,
        episode_id: EpisodeID<'static>,
        // Potentially can include link to channel.
        thumbnails: Vec<Thumbnail>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
/// A profile search result.
pub struct SearchResultProfile {
    pub title: String,
    pub username: String,
    pub profile_id: UserChannelID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
/// An album search result.
pub struct SearchResultAlbum {
    pub title: String,
    pub artist: String,
    pub year: String,
    pub explicit: Explicit,
    pub album_id: AlbumID<'static>,
    pub album_type: AlbumType,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SearchResultSong {
    // Potentially can include links to artist and album.
    pub title: String,
    pub artist: String,
    // Album field can be optional - see https://github.com/nick42d/youtui/issues/174
    pub album: Option<ParsedSongAlbum>,
    pub duration: String,
    pub plays: String,
    pub explicit: Explicit,
    pub video_id: VideoID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
// A playlist search result may be a featured or community playlist or even a
// podcast.
pub enum SearchResultPlaylist {
    Featured(SearchResultFeaturedPlaylist),
    Community(SearchResultCommunityPlaylist),
    Podcast(SearchResultPodcast),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
// When doing a basic search, community playlists might actually be podcasts.
pub enum BasicSearchResultCommunityPlaylist {
    Podcast(SearchResultPodcast),
    Playlist(SearchResultCommunityPlaylist),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
/// A community playlist search result.
pub struct SearchResultCommunityPlaylist {
    pub title: String,
    pub author: String,
    pub views: String,
    pub playlist_id: PlaylistID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
/// A featured playlist search result.
pub struct SearchResultFeaturedPlaylist {
    pub title: String,
    pub author: String,
    pub songs: String,
    pub playlist_id: PlaylistID<'static>,
    pub thumbnails: Vec<Thumbnail>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct MetadataSegment {
    text: String,
    browse_ids: Vec<String>,
}

impl MetadataSegment {
    fn trimmed_text(&self) -> String {
        self.text.trim().to_string()
    }

    fn first_browse_id_with_prefix(&self, prefixes: &[&str]) -> Option<String> {
        self.browse_ids
            .iter()
            .find(|id| prefixes.is_empty() || prefixes.iter().any(|prefix| id.starts_with(prefix)))
            .cloned()
    }
}

fn parse_flex_column_metadata_segments(
    item: &mut impl JsonCrawler,
    col_idx: usize,
) -> Result<Vec<MetadataSegment>> {
    let mut runs =
        item.borrow_pointer(format!("{}/text/runs", flex_column_item_pointer(col_idx)))?;
    let mut segments = Vec::new();
    let mut current = MetadataSegment::default();

    for run in runs.try_iter_mut()? {
        let text: String = run.borrow_value_pointer("/text")?;
        if text == " • " {
            if !current.text.trim().is_empty() {
                segments.push(current);
                current = MetadataSegment::default();
            }
            continue;
        }

        current.text.push_str(&text);
        if let Ok(browse_id) = run.borrow_value_pointer::<String>(NAVIGATION_BROWSE_ID) {
            current.browse_ids.push(browse_id);
        }
    }

    if !current.text.trim().is_empty() {
        segments.push(current);
    }

    Ok(segments)
}

fn find_item_browse_id(item: &mut impl JsonCrawler, prefixes: &[&str]) -> Option<String> {
    if let Ok(id) = item.borrow_value_pointer::<String>(NAVIGATION_BROWSE_ID) {
        if prefixes.is_empty() || prefixes.iter().any(|prefix| id.starts_with(prefix)) {
            return Some(id);
        }
    }

    for col_idx in 0..4 {
        if let Ok(segments) = parse_flex_column_metadata_segments(item, col_idx) {
            if let Some(id) = segments
                .into_iter()
                .find_map(|segment| segment.first_browse_id_with_prefix(prefixes))
            {
                return Some(id);
            }
        }
    }

    None
}

fn is_duration_text(text: &str) -> bool {
    let parts = text.trim().split(':').collect_vec();
    (2..=3).contains(&parts.len())
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
}

fn is_song_metadata_label(text: &str) -> bool {
    matches!(text.trim(), "Song")
}

fn parse_album_type_label(text: &str) -> Option<AlbumType> {
    match text.trim().to_lowercase().as_str() {
        "album" => Some(AlbumType::Album),
        "single" => Some(AlbumType::Single),
        "ep" => Some(AlbumType::EP),
        _ => None,
    }
}

fn join_artist_segments(segments: &[MetadataSegment]) -> String {
    segments
        .iter()
        .map(MetadataSegment::trimmed_text)
        .filter(|text| !text.is_empty())
        .join(" & ")
}

fn parse_song_metadata(
    mrlir: &mut impl JsonCrawler,
) -> Result<(String, Option<ParsedSongAlbum>, String)> {
    let segments = parse_flex_column_metadata_segments(mrlir, 1)?;
    let filtered_segments = segments
        .into_iter()
        .filter(|segment| !is_song_metadata_label(&segment.text))
        .collect_vec();

    let duration_idx = filtered_segments
        .iter()
        .rposition(|segment| is_duration_text(&segment.text));
    let duration = duration_idx
        .and_then(|idx| filtered_segments.get(idx))
        .map(MetadataSegment::trimmed_text)
        .unwrap_or_default();

    let metadata_segments = filtered_segments
        .into_iter()
        .enumerate()
        .filter_map(|(idx, segment)| (Some(idx) != duration_idx).then_some(segment))
        .collect_vec();

    let album_idx = metadata_segments
        .iter()
        .rposition(|segment| segment.first_browse_id_with_prefix(&["MP"]).is_some());

    let album = album_idx
        .filter(|idx| *idx > 0)
        .and_then(|idx| metadata_segments.get(idx))
        .and_then(|segment| {
            segment
                .first_browse_id_with_prefix(&["MP"])
                .map(|browse_id| ParsedSongAlbum {
                    name: segment.trimmed_text(),
                    id: AlbumID::from_raw(browse_id),
                })
        });

    let artist_segments = album_idx
        .filter(|idx| *idx > 0)
        .map(|idx| &metadata_segments[..idx])
        .unwrap_or(&metadata_segments[..]);
    let artist = join_artist_segments(artist_segments);

    Ok((artist, album, duration))
}

fn parse_untyped_top_result_metadata(
    mrlir: &mut impl JsonCrawler,
) -> Result<(
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
)> {
    let segments = parse_flex_column_metadata_segments(mrlir, 1)?;
    let duration_idx = segments
        .iter()
        .rposition(|segment| is_duration_text(&segment.text));
    let duration = duration_idx
        .and_then(|idx| segments.get(idx))
        .map(MetadataSegment::trimmed_text)
        .filter(|text| !text.is_empty());

    let metadata_segments = segments
        .into_iter()
        .enumerate()
        .filter_map(|(idx, segment)| (Some(idx) != duration_idx).then_some(segment))
        .collect_vec();

    if metadata_segments.is_empty() {
        return Ok((None, None, duration, None));
    }

    let album_idx = metadata_segments
        .iter()
        .rposition(|segment| segment.first_browse_id_with_prefix(&["MP"]).is_some());

    if let Some(idx) = album_idx.filter(|idx| *idx > 0) {
        let artist = join_artist_segments(&metadata_segments[..idx]);
        let album = metadata_segments[idx].trimmed_text();
        return Ok((
            (!artist.is_empty()).then_some(artist),
            (!album.is_empty()).then_some(album),
            duration,
            None,
        ));
    }

    let artist = metadata_segments
        .first()
        .map(MetadataSegment::trimmed_text)
        .filter(|text| !text.is_empty());
    let secondary = metadata_segments
        .get(1)
        .map(MetadataSegment::trimmed_text)
        .filter(|text| !text.is_empty());

    let is_plays_like = |text: &str| {
        let lower = text.trim().to_ascii_lowercase();
        lower.contains("views")
            || lower.contains("monthly audience")
            || lower.contains("subscribers")
            || lower.contains("plays")
    };

    let (album, plays) = match secondary {
        Some(text) if is_plays_like(&text) => (None, Some(text)),
        Some(text) => (Some(text), None),
        None => (None, None),
    };

    Ok((artist, album, duration, plays))
}

// TODO: Type safety
fn parse_basic_search_result_from_section_list_contents(
    mut section_list_contents: BasicSearchSectionListContents,
) -> Result<SearchResults> {
    // Imperative solution, may be able to make more functional.
    let mut top_results = Vec::new();
    let mut artists = Vec::new();
    let mut albums = Vec::new();
    let mut featured_playlists = Vec::new();
    let mut community_playlists = Vec::new();
    let mut songs = Vec::new();
    let mut videos = Vec::new();
    let mut podcasts = Vec::new();
    let mut episodes = Vec::new();
    let mut profiles = Vec::new();

    let music_card_shelf = section_list_contents
        .0
        .try_iter_mut()?
        .find_path(MUSIC_CARD_SHELF)
        .ok();
    if let Some(music_card_shelf) = music_card_shelf {
        top_results = parse_top_results_from_music_card_shelf_contents(music_card_shelf)?
    }
    let results_iter = section_list_contents
        .0
        .try_into_iter()?
        .filter_map(|item| item.navigate_pointer(MUSIC_SHELF).ok());

    for mut category in results_iter {
        match category.take_value_pointer::<SearchResultType>(TITLE_TEXT)? {
            SearchResultType::TopResult => {
                top_results = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .filter_map(|r| parse_top_result_from_music_shelf_contents(r).transpose())
                    .collect::<Result<Vec<TopResult>>>()?;
            }
            // TODO: Use a navigation constant
            SearchResultType::Artists => {
                artists = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .map(|r| parse_artist_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultArtist>>>()?;
            }
            SearchResultType::Albums => {
                albums = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .map(|r| parse_album_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultAlbum>>>()?
            }
            SearchResultType::FeaturedPlaylists => {
                featured_playlists = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .map(|r| parse_featured_playlist_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultFeaturedPlaylist>>>()?
            }
            SearchResultType::CommunityPlaylists => {
                community_playlists = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .map(|r| {
                        parse_community_playlist_basic_search_result_from_music_shelf_contents(r)
                    })
                    .collect::<Result<Vec<BasicSearchResultCommunityPlaylist>>>()?
            }
            SearchResultType::Songs => {
                songs = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .map(|r| parse_song_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultSong>>>()?
            }
            SearchResultType::Videos => {
                videos = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .filter_map(|r| {
                        parse_video_search_result_from_music_shelf_contents(r).transpose()
                    })
                    .collect::<Result<Vec<SearchResultVideo>>>()?
            }
            SearchResultType::Podcasts => {
                podcasts = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .map(|r| parse_podcast_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultPodcast>>>()?
            }
            SearchResultType::Episodes => {
                episodes = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .map(|r| parse_episode_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultEpisode>>>()?
            }
            SearchResultType::Profiles => {
                profiles = category
                    .navigate_pointer("/contents")?
                    .try_iter_mut()?
                    .map(|r| parse_profile_search_result_from_music_shelf_contents(r))
                    .collect::<Result<Vec<SearchResultProfile>>>()?
            }
            SearchResultType::Unknown => {
                // Silently skip unknown section types (e.g. "Listen again", future YTM additions)
            }
            SearchResultType::MoreResults => {
                if let Ok(mut contents) = category.navigate_pointer("/contents") {
                    if let Ok(items) = contents.try_iter_mut() {
                        for item in items {
                            let has_watch = item.path_exists(
                                "/musicResponsiveListItemRenderer/navigationEndpoint/watchEndpoint",
                            );
                            let _has_browse = item.path_exists("/musicResponsiveListItemRenderer/navigationEndpoint/browseEndpoint");

                            if has_watch {
                                // It's a Song or Video
                                // Prioritize Song parsing (our relaxed parser handles most cases)
                                if let Ok(song) =
                                    parse_song_search_result_from_music_shelf_contents(item)
                                {
                                    songs.push(song);
                                }
                            } else {
                                let browse_id = item.borrow_value_pointer::<String>("/musicResponsiveListItemRenderer/navigationEndpoint/browseEndpoint/browseId");

                                if let Ok(id) = browse_id {
                                    if id.starts_with("UC") {
                                        match parse_artist_search_result_from_music_shelf_contents(
                                            item,
                                        ) {
                                            Ok(artist) => artists.push(artist),
                                            Err(_) => {} // Ignore parse errors
                                        }
                                    } else if id.starts_with("MP") {
                                        match parse_album_search_result_from_music_shelf_contents(
                                            item,
                                        ) {
                                            Ok(album) => albums.push(album),
                                            Err(_) => {} // Ignore parse errors
                                        }
                                    } else if id.starts_with("VL") || id.starts_with("PL") {
                                        if let Ok(playlist) = parse_community_playlist_search_result_from_music_shelf_contents(item) {
                                            community_playlists.push(BasicSearchResultCommunityPlaylist::Playlist(playlist));
                                        }
                                    }
                                } else {
                                    // No watch, no browse. Likely a Song with different structure (e.g. playlistItemData).
                                    // Fallback to Song parsing.
                                    if let Ok(song) =
                                        parse_song_search_result_from_music_shelf_contents(item)
                                    {
                                        songs.push(song);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(SearchResults {
        top_results,
        artists,
        albums,
        featured_playlists,
        community_playlists,
        songs,
        videos,
        podcasts,
        episodes,
        profiles,
    })
}

fn parse_top_results_from_music_card_shelf_contents(
    mut music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<Vec<TopResult>> {
    let mut results = Vec::new();
    // Begin - first result parsing
    let result_name = music_shelf_contents.take_value_pointer(TITLE_TEXT)?;
    let subtitle: Option<String> = music_shelf_contents.take_value_pointer(SUBTITLE).ok();
    let result_type = subtitle
        .as_ref()
        .map(|s| TopResultType::from_subtitle(s))
        .unwrap_or_else(|| TopResultType::Unknown("missing".to_string()));
    let subtitle_2: Option<String> = music_shelf_contents.take_value_pointer(SUBTITLE2).ok();

    let thumbnails: Vec<Thumbnail> = music_shelf_contents.take_value_pointer(THUMBNAILS)?;
    let browse_id = music_shelf_contents
        .take_value_pointer(NAVIGATION_BROWSE_ID)
        .ok()
        .or_else(|| {
            music_shelf_contents
                .take_value_pointer("/title/runs/0/navigationEndpoint/browseEndpoint/browseId")
                .ok()
        });

    // Extract video_id once, then use is_some() for artist/subscribers decision
    let video_id = music_shelf_contents
        .take_value_pointer(ON_TAP_VIDEO_ID)
        .ok()
        .or_else(|| {
            music_shelf_contents
                .take_value_pointer(TITLE_NAV_VIDEO_ID)
                .ok()
        })
        .or_else(|| {
            music_shelf_contents
                .take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)
                .ok()
        });

    let (artist, subscribers) = if video_id.is_some() {
        (subtitle_2, None)
    } else {
        (None, subtitle_2)
    };

    let byline = match &result_type {
        TopResultType::Unknown(_) => subtitle.clone(),
        _ => None,
    };
    let publisher = None;
    let album = None;
    let duration = None;
    let year = None;
    let plays = None;
    let first_result = TopResult {
        result_type: Some(result_type),
        subscribers,
        thumbnails,
        result_name,
        publisher,
        artist,
        album,
        duration,
        year,
        plays,
        byline,
        browse_id,
        video_id,
    };
    // End - first result parsing.
    results.push(first_result);
    // Other results may not exist.
    if let Ok(mut contents) = music_shelf_contents.navigate_pointer("/contents") {
        contents
            .try_iter_mut()?
            .filter_map(|r| parse_top_result_from_music_shelf_contents(r).transpose())
            .try_for_each(|r| -> Result<()> {
                results.push(r?);
                Ok(())
            })?;
    }
    Ok(results)
}
// TODO: Tests
fn parse_top_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<Option<TopResult>> {
    // This is the "More from YouTube" seperator
    if music_shelf_contents.path_exists("/messageRenderer") {
        return Ok(None);
    };
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let root_browse_id = mrlir.borrow_value_pointer(NAVIGATION_BROWSE_ID).ok();
    let artist_browse_id = find_item_browse_id(&mut mrlir, &["UC"]);
    let album_browse_id = find_item_browse_id(&mut mrlir, &["MP"]);
    let playlist_browse_id = find_item_browse_id(&mut mrlir, &["VL", "PL", "RD"]);
    let result_name = parse_flex_column_item(&mut mrlir, 0, 0)?;
    // It's possible to have artist name in the first position instead of a
    // TopResultType. There may be a way to differentiate this even further.
    let flex_1_0: String =
        mrlir.borrow_value_pointer(format!("{}/text/runs/0/text", flex_column_item_pointer(1)))?;
    // Deserialize without taking ownership of flex_1_0 - not possible with
    // JsonCrawler::take_value_pointer().
    // TODO: add methods like borrow_value_pointer() to JsonCrawler.
    let result_type_result: std::result::Result<_, serde::de::value::Error> =
        TopResultType::deserialize(flex_1_0.as_str().into_deserializer());
    let result_type = result_type_result.ok();
    // Imperative solution, may be able to make more functional.
    let mut subscribers = None;
    let mut publisher = None;
    let mut artist = None;
    let mut album = None;
    let mut duration = None;
    let mut year = None;
    let mut plays = None;
    match result_type {
        // XXX: Perhaps also populate Artist field.
        Some(TopResultType::Artist) => {
            subscribers = Some(parse_flex_column_item(&mut mrlir, 1, 2)?)
        }
        Some(TopResultType::Album(_)) => {
            // XXX: Perhaps also populate Album field.
            artist = Some(parse_flex_column_item(&mut mrlir, 1, 2)?);
            year = Some(parse_flex_column_item(&mut mrlir, 1, 4)?);
        }
        Some(TopResultType::Playlist) => {
            // Python: author from flex_column_item(1, 2), similar to artist structure
            // Playlists show: title (already in result_name), author at 1,2
            artist = parse_flex_column_item(&mut mrlir, 1, 2).ok();
        }
        Some(TopResultType::Song) => {
            let (parsed_artist, parsed_album, parsed_duration) = parse_song_metadata(&mut mrlir)?;
            artist = (!parsed_artist.is_empty()).then_some(parsed_artist);
            album = parsed_album.map(|album| album.name);
            duration = (!parsed_duration.is_empty()).then_some(parsed_duration);
            // This does not show up in all Card renderer results and so we'll define it as
            // optional. TODO: Could make this more type safe in future.
            plays = parse_flex_column_item(&mut mrlir, 2, 0)
                .ok()
                .or_else(|| parse_flex_column_item(&mut mrlir, 1, 8).ok());
        }
        Some(TopResultType::Video) => {
            // Python: artist/channel at flex(1,2), duration at flex(1,4)
            // Videos show: title (in result_name), channel, duration
            artist = parse_flex_column_item(&mut mrlir, 1, 2).ok();
            duration = parse_flex_column_item(&mut mrlir, 1, 4).ok();
        }
        Some(TopResultType::Station) => {
            // Python: station has videoId and playlistId for radio functionality
            // Stations show: title (in result_name), subscriber-like info
            // Station is like an auto-generated playlist/radio
            subscribers = parse_flex_column_item(&mut mrlir, 1, 2).ok();
        }
        Some(TopResultType::Podcast) => publisher = Some(parse_flex_column_item(&mut mrlir, 1, 2)?),
        Some(TopResultType::Unknown(_)) => {
            let (parsed_artist, parsed_album, parsed_duration, parsed_plays) =
                parse_untyped_top_result_metadata(&mut mrlir)?;
            artist = parsed_artist.or(Some(flex_1_0));
            album = parsed_album;
            duration = parsed_duration;
            plays = parsed_plays.or_else(|| parse_flex_column_item(&mut mrlir, 2, 0).ok());
        }
        None => {
            let (parsed_artist, parsed_album, parsed_duration, parsed_plays) =
                parse_untyped_top_result_metadata(&mut mrlir)?;
            artist = parsed_artist.or(Some(flex_1_0));
            album = parsed_album;
            duration = parsed_duration;
            // This does not show up in all Card renderer results and so we'll define it as
            // optional. TODO: Could make this more type safe in future.
            plays = parsed_plays
                .or_else(|| parse_flex_column_item(&mut mrlir, 2, 0).ok())
                .or_else(|| parse_flex_column_item(&mut mrlir, 1, 6).ok());
        }
    }
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    let browse_id = match &result_type {
        Some(TopResultType::Artist) => artist_browse_id.or(root_browse_id.clone()),
        Some(TopResultType::Album(_)) => album_browse_id.or(root_browse_id.clone()),
        Some(TopResultType::Playlist) | Some(TopResultType::Station) => {
            playlist_browse_id.or(root_browse_id.clone())
        }
        _ => root_browse_id,
    };
    let video_id = mrlir.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID).ok();
    Ok(Some(TopResult {
        result_type,
        subscribers,
        thumbnails,
        result_name,
        publisher,
        artist,
        album,
        duration,
        year,
        plays,
        byline: None,
        browse_id,
        video_id,
    }))
}
// TODO: Type safety
// TODO: Tests
fn parse_artist_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultArtist> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let browse_id = find_item_browse_id(&mut mrlir, &["UC"])
        .map(|browse_id| ArtistChannelID::from_raw(browse_id))
        .ok_or_else(|| {
            Error::other_code(0, "Artist search result missing browse id".to_string())
        })?;
    let artist = parse_flex_column_item(&mut mrlir, 0, 0)
        .ok()
        .unwrap_or_else(|| "Unknown Artist".to_string());
    let subscribers = parse_flex_column_item(&mut mrlir, 1, 2).ok();
    let thumbnails: Vec<Thumbnail> = mrlir
        .take_value_pointer(THUMBNAILS)
        .ok()
        .unwrap_or_default();
    Ok(SearchResultArtist {
        artist,
        subscribers,
        thumbnails,
        browse_id,
    })
}

// TODO: Type safety
// TODO: Tests
fn parse_profile_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultProfile> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let title = parse_flex_column_item(&mut mrlir, 0, 0)?;
    let username = parse_flex_column_item(&mut mrlir, 1, 2)?;
    let profile_id = mrlir.take_value_pointer(NAVIGATION_BROWSE_ID)?;
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(SearchResultProfile {
        title,
        username,
        profile_id,
        thumbnails,
    })
}

// TODO: Type safety
// TODO: Tests
fn parse_album_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultAlbum> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let browse_id = find_item_browse_id(&mut mrlir, &["MP"])
        .map(|browse_id| AlbumID::from_raw(browse_id))
        .ok_or_else(|| Error::other_code(0, "Album search result missing browse id".to_string()))?;
    let metadata_segments = parse_flex_column_metadata_segments(&mut mrlir, 1).unwrap_or_default();
    let title = parse_flex_column_item(&mut mrlir, 0, 0)
        .ok()
        .unwrap_or_else(|| "Unknown Album".to_string());
    let album_type = metadata_segments
        .first()
        .and_then(|segment| parse_album_type_label(&segment.text))
        .unwrap_or(AlbumType::Album);
    let detail_segments = if metadata_segments
        .first()
        .and_then(|segment| parse_album_type_label(&segment.text))
        .is_some()
    {
        &metadata_segments[1..]
    } else {
        &metadata_segments[..]
    };
    let artist = detail_segments
        .first()
        .map(MetadataSegment::trimmed_text)
        .filter(|artist| !artist.is_empty())
        .unwrap_or_else(|| "Unknown Artist".to_string());
    let year = detail_segments
        .get(1)
        .map(MetadataSegment::trimmed_text)
        .filter(|year| !year.is_empty())
        .unwrap_or_else(|| "Unknown Year".to_string());

    let explicit = if mrlir.path_exists(BADGE_LABEL) {
        Explicit::IsExplicit
    } else {
        Explicit::NotExplicit
    };
    let thumbnails: Vec<Thumbnail> = mrlir
        .take_value_pointer(THUMBNAILS)
        .ok()
        .unwrap_or_default();
    Ok(SearchResultAlbum {
        artist,
        thumbnails,
        album_id: browse_id,
        title,
        year,
        album_type,
        explicit,
    })
}
fn parse_song_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultSong> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let title = parse_flex_column_item(&mut mrlir, 0, 0)?;
    let (artist, album, duration) = parse_song_metadata(&mut mrlir)?;

    // Make plays optional (index 2 might be missing in More Results)
    let plays = parse_flex_column_item(&mut mrlir, 2, 0).unwrap_or_default();

    let explicit = if mrlir.path_exists(BADGE_LABEL) {
        Explicit::IsExplicit
    } else {
        Explicit::NotExplicit
    };
    let video_id = mrlir.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)?;
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(SearchResultSong {
        artist,
        thumbnails,
        title,
        explicit,
        plays,
        album,
        video_id,
        duration,
    })
}
// TODO: Type safety
// TODO: Tests
fn parse_video_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<Option<SearchResultVideo>> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    // Handle not available case
    if let Ok("MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT") = mrlir
        .take_value_pointer::<String>(DISPLAY_POLICY)
        .as_deref()
    {
        return Ok(None);
    };
    let title = parse_flex_column_item(&mut mrlir, 0, 0)?;
    let first_field: String = parse_flex_column_item(&mut mrlir, 1, 0)?;
    // Handle video podcasts - seems to be 2 different ways to display these.
    match first_field.as_str() {
        "Video" => {
            let channel_name = parse_flex_column_item(&mut mrlir, 1, 2)?;
            let views = parse_flex_column_item(&mut mrlir, 1, 4)?;
            let length = parse_flex_column_item(&mut mrlir, 1, 6)?;
            let video_id = mrlir.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)?;
            let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
            Ok(Some(SearchResultVideo::Video {
                title,
                channel_name,
                views,
                length,
                thumbnails,
                video_id,
            }))
        }
        "Episode" => {
            //TODO: Handle live episode
            let date = EpisodeDate::Recorded {
                date: parse_flex_column_item(&mut mrlir, 1, 2)?,
            };
            let channel_name = parse_flex_column_item(&mut mrlir, 1, 4)?;
            let video_id = mrlir.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)?;
            let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
            Ok(Some(SearchResultVideo::VideoEpisode {
                title,
                channel_name,
                date,
                thumbnails,
                episode_id: video_id,
            }))
        }
        _ => {
            // Assume that if a watch endpoint exists, it's a video.
            if mrlir.path_exists("/flexColumns/0/musicResponsiveListItemFlexColumnRenderer/text/runs/0/navigationEndpoint/watchEndpoint") {

            let views = parse_flex_column_item(&mut mrlir, 1, 2)?;
            let length = parse_flex_column_item(&mut mrlir, 1, 4)?;
            let video_id = mrlir.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)?;
            let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
            Ok(Some(SearchResultVideo::Video {
                            title,
                            channel_name: first_field,
                            views,
                            length,
                            thumbnails,
                            video_id,
                        }))
            } else {
            let channel_name = parse_flex_column_item(&mut mrlir, 1, 2)?;
            let video_id = mrlir.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)?;
            let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
            Ok(Some(SearchResultVideo::VideoEpisode {
                            title,
                            channel_name,
                        //TODO: Handle live episode
                            date: EpisodeDate::Recorded { date: first_field },
                            thumbnails,
                            episode_id: video_id,
                        }))
            }
        }
    }
}
// TODO: Type safety
// TODO: Tests
fn parse_podcast_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultPodcast> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let title = parse_flex_column_item(&mut mrlir, 0, 0)?;
    let publisher = parse_flex_column_item(&mut mrlir, 1, 0)?;
    let podcast_id = mrlir.take_value_pointer(NAVIGATION_BROWSE_ID)?;
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(SearchResultPodcast {
        title,
        publisher,
        podcast_id,
        thumbnails,
    })
}
// TODO: Type safety
// TODO: Tests
fn parse_episode_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultEpisode> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let title = parse_flex_column_item(&mut mrlir, 0, 0)?;
    let date = if mrlir.path_exists(LIVE_BADGE_LABEL) {
        EpisodeDate::Live
    } else {
        EpisodeDate::Recorded {
            date: parse_flex_column_item(&mut mrlir, 1, 0)?,
        }
    };
    let channel_name = match date {
        EpisodeDate::Live => parse_flex_column_item(&mut mrlir, 1, 0)?,
        EpisodeDate::Recorded { .. } => parse_flex_column_item(&mut mrlir, 1, 2)?,
    };
    let video_id = mrlir.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)?;
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(SearchResultEpisode {
        title,
        date,
        episode_id: video_id,
        channel_name,
        thumbnails,
    })
}
// TODO: Type safety
// TODO: Tests
fn parse_featured_playlist_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultFeaturedPlaylist> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let playlist_id = find_item_browse_id(&mut mrlir, &["VL", "PL", "RD"])
        .map(|browse_id| PlaylistID::from_raw(browse_id))
        .ok_or_else(|| {
            Error::other_code(
                0,
                "Featured playlist search result missing browse id".to_string(),
            )
        })?;
    let metadata_segments = parse_flex_column_metadata_segments(&mut mrlir, 1).unwrap_or_default();
    let detail_segments = if metadata_segments
        .first()
        .map(|segment| segment.trimmed_text() == "Playlist")
        .unwrap_or(false)
    {
        &metadata_segments[1..]
    } else {
        &metadata_segments[..]
    };
    let title = parse_flex_column_item(&mut mrlir, 0, 0)?;
    let author = detail_segments
        .first()
        .map(MetadataSegment::trimmed_text)
        .unwrap_or_default();
    let songs = detail_segments
        .get(1)
        .map(MetadataSegment::trimmed_text)
        .unwrap_or_default();
    let thumbnails: Vec<Thumbnail> = mrlir.take_value_pointer(THUMBNAILS)?;
    Ok(SearchResultFeaturedPlaylist {
        title,
        author,
        playlist_id,
        songs,
        thumbnails,
    })
}
fn parse_community_playlist_basic_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<BasicSearchResultCommunityPlaylist> {
    let result_type: YoutubeMusicPageType = music_shelf_contents
        .borrow_value_pointer(concatcp!(MRLIR, NAVIGATION_BROWSE, PAGE_TYPE))?;
    let result = match result_type {
        YoutubeMusicPageType::Podcast => BasicSearchResultCommunityPlaylist::Podcast(
            parse_podcast_search_result_from_music_shelf_contents(music_shelf_contents)?,
        ),
        YoutubeMusicPageType::Playlist => BasicSearchResultCommunityPlaylist::Playlist(
            parse_community_playlist_search_result_from_music_shelf_contents(music_shelf_contents)?,
        ),
    };
    Ok(result)
}
fn parse_community_playlist_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultCommunityPlaylist> {
    let mut mrlir = music_shelf_contents.navigate_pointer("/musicResponsiveListItemRenderer")?;
    let playlist_id = find_item_browse_id(&mut mrlir, &["VL", "PL", "RD"])
        .map(|browse_id| PlaylistID::from_raw(browse_id))
        .ok_or_else(|| {
            Error::other_code(
                0,
                "Community playlist search result missing browse id".to_string(),
            )
        })?;
    let metadata_segments = parse_flex_column_metadata_segments(&mut mrlir, 1).unwrap_or_default();
    let detail_segments = if metadata_segments
        .first()
        .map(|segment| segment.trimmed_text() == "Playlist")
        .unwrap_or(false)
    {
        &metadata_segments[1..]
    } else {
        &metadata_segments[..]
    };
    let title = parse_flex_column_item(&mut mrlir, 0, 0)?;
    let author = detail_segments
        .first()
        .map(MetadataSegment::trimmed_text)
        .unwrap_or_default();
    let views = detail_segments
        .get(1)
        .map(MetadataSegment::trimmed_text)
        .unwrap_or_default();
    let thumbnails: Vec<Thumbnail> = mrlir
        .take_value_pointer(THUMBNAILS)
        .ok()
        .unwrap_or_default();
    Ok(SearchResultCommunityPlaylist {
        title,
        author,
        playlist_id,
        views,
        thumbnails,
    })
}

fn parse_playlist_search_result_from_music_shelf_contents(
    mut music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultPlaylist> {
    let result_type: YoutubeMusicPageType = music_shelf_contents
        .borrow_value_pointer(concatcp!(MRLIR, NAVIGATION_BROWSE, PAGE_TYPE))?;

    // Search result for this query can be Podcast or Playlist.
    match result_type {
        YoutubeMusicPageType::Podcast => {
            let res = parse_podcast_search_result_from_music_shelf_contents(music_shelf_contents)?;
            Ok(SearchResultPlaylist::Podcast(res))
        }
        YoutubeMusicPageType::Playlist => {
            // The playlist search contains a mix of Community and Featured playlists.
            let playlist_params: PlaylistEndpointParams =
                music_shelf_contents.take_value_pointer(concatcp!(
                    MRLIR,
                    PLAY_BUTTON,
                    "/playNavigationEndpoint/watchPlaylistEndpoint/params"
                ))?;
            let playlist = match playlist_params {
                PlaylistEndpointParams::Featured => {
                    let res = parse_featured_playlist_search_result_from_music_shelf_contents(
                        music_shelf_contents,
                    )?;
                    SearchResultPlaylist::Featured(res)
                }
                PlaylistEndpointParams::Community => {
                    let res = parse_community_playlist_search_result_from_music_shelf_contents(
                        music_shelf_contents,
                    )?;
                    SearchResultPlaylist::Community(res)
                }
            };
            Ok(playlist)
        }
    }
}

struct FilteredSearchSectionContents(JsonCrawlerOwned);
struct FilteredSearchMusicShelfContents(JsonCrawlerOwned);
struct BasicSearchSectionListContents(JsonCrawlerOwned);
// In this case, we've searched and had no results found.
// We are being quite explicit here to avoid a false positive.
// See tests for an example.
// TODO: Test this function itself.
fn section_contents_is_empty(section_contents: &mut FilteredSearchSectionContents) -> Result<bool> {
    Ok(section_contents
        .0
        .try_iter_mut()?
        .any(|item| item.path_exists("/itemSectionRenderer/contents/0/didYouMeanRenderer")))
}

fn take_continuation_params_from_section_contents(
    section_contents: &mut FilteredSearchSectionContents,
) -> Result<Option<ContinuationParams<'static>>> {
    section_contents
        .0
        .try_iter_mut()
        .and_then(|contents| contents.find_path(concatcp!(MUSIC_SHELF, CONTINUATION_PARAMS)))
        .map(|mut continuation_params| continuation_params.take_value())
        .ok()
        .transpose()
        .map_err(Into::into)
}
fn get_filtered_search_continuation_music_shelf_contents_and_params(
    crawler: JsonCrawlerOwned,
) -> Result<(
    FilteredSearchMusicShelfContents,
    Option<ContinuationParams<'static>>,
)> {
    let mut music_shelf = crawler.navigate_pointer(MUSIC_SHELF_CONTINUATION)?;
    let continuation_params = music_shelf.take_value_pointer(CONTINUATION_PARAMS).ok();
    let contents = music_shelf.navigate_pointer("/contents")?;
    Ok((
        FilteredSearchMusicShelfContents(contents),
        continuation_params,
    ))
}
// TODO: Consolidate these two functions into single function.
// TODO: This could be implemented with a non-mutable array also.
fn section_list_contents_is_empty(
    section_contents: &mut BasicSearchSectionListContents,
) -> Result<bool> {
    let is_empty = section_contents
        .0
        .try_iter_mut()?
        .filter(|item| item.path_exists(MUSIC_CARD_SHELF) || item.path_exists(MUSIC_SHELF))
        .count()
        == 0;
    Ok(is_empty)
}
impl<'a, S: UnfilteredSearchType> TryFrom<ProcessedResult<'a, SearchQuery<'a, S>>>
    for BasicSearchSectionListContents
{
    type Error = Error;
    fn try_from(value: ProcessedResult<SearchQuery<'a, S>>) -> Result<Self> {
        let json_crawler: JsonCrawlerOwned = value.into();
        let section_list_contents = json_crawler.navigate_pointer(concatcp!(
            "/contents/tabbedSearchResultsRenderer",
            TAB_CONTENT,
            SECTION_LIST
        ))?;
        Ok(BasicSearchSectionListContents(section_list_contents))
    }
}
impl<'a, F: FilteredSearchType> TryFrom<ProcessedResult<'a, SearchQuery<'a, FilteredSearch<F>>>>
    for FilteredSearchSectionContents
{
    type Error = Error;
    fn try_from(value: ProcessedResult<SearchQuery<'a, FilteredSearch<F>>>) -> Result<Self> {
        let json_crawler: JsonCrawlerOwned = value.into();
        let section_contents = json_crawler.navigate_pointer(concatcp!(
            "/contents/tabbedSearchResultsRenderer",
            TAB_CONTENT,
            SECTION_LIST,
        ))?;
        Ok(FilteredSearchSectionContents(section_contents))
    }
}
impl TryFrom<FilteredSearchSectionContents> for FilteredSearchMusicShelfContents {
    type Error = Error;
    fn try_from(
        value: FilteredSearchSectionContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        let music_shelf_contents = value
            .0
            .try_into_iter()?
            .find_path(concatcp!(MUSIC_SHELF, "/contents"))?;
        Ok(FilteredSearchMusicShelfContents(music_shelf_contents))
    }
}
impl TryFrom<FilteredSearchMusicShelfContents> for Vec<SearchResultAlbum> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMusicShelfContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .map(|a| parse_album_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl TryFrom<FilteredSearchMusicShelfContents> for Vec<SearchResultProfile> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMusicShelfContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .map(|a| parse_profile_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl TryFrom<FilteredSearchMusicShelfContents> for Vec<SearchResultArtist> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMusicShelfContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .map(|a| parse_artist_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl TryFrom<FilteredSearchMusicShelfContents> for Vec<SearchResultSong> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMusicShelfContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .map(|a| parse_song_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl TryFrom<FilteredSearchMusicShelfContents> for Vec<SearchResultVideo> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMusicShelfContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .filter_map(|a| parse_video_search_result_from_music_shelf_contents(a).transpose())
            .collect()
    }
}
impl TryFrom<FilteredSearchMusicShelfContents> for Vec<SearchResultEpisode> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMusicShelfContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .map(|a| parse_episode_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl TryFrom<FilteredSearchMusicShelfContents> for Vec<SearchResultPodcast> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMusicShelfContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .map(|a| parse_podcast_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl TryFrom<FilteredSearchMusicShelfContents> for Vec<SearchResultPlaylist> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMusicShelfContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .map(|a| parse_playlist_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl TryFrom<FilteredSearchMusicShelfContents> for Vec<SearchResultCommunityPlaylist> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMusicShelfContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .map(|a| parse_community_playlist_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl TryFrom<FilteredSearchMusicShelfContents> for Vec<SearchResultFeaturedPlaylist> {
    type Error = Error;
    fn try_from(
        mut value: FilteredSearchMusicShelfContents,
    ) -> std::prelude::v1::Result<Self, Self::Error> {
        // TODO: Make this a From method.
        value
            .0
            .try_iter_mut()?
            .map(|a| parse_featured_playlist_search_result_from_music_shelf_contents(a))
            .collect()
    }
}
impl<'a, S: UnfilteredSearchType> ParseFrom<SearchQuery<'a, S>> for SearchResults {
    fn parse_from(p: ProcessedResult<SearchQuery<'a, S>>) -> crate::Result<Self> {
        let mut section_list_contents = BasicSearchSectionListContents::try_from(p)?;
        if section_list_contents_is_empty(&mut section_list_contents)? {
            return Ok(Self::default());
        }
        parse_basic_search_result_from_section_list_contents(section_list_contents)
    }
}

impl<'a> ParseFromContinuable<SearchQuery<'a, FilteredSearch<ArtistsFilter>>>
    for Vec<SearchResultArtist>
{
    fn parse_from_continuable(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<ArtistsFilter>>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let mut section_contents = FilteredSearchSectionContents::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok((Vec::new(), None));
        }
        let continuation_params =
            take_continuation_params_from_section_contents(&mut section_contents)?;
        let results = FilteredSearchMusicShelfContents::try_from(section_contents)?.try_into()?;
        Ok((results, continuation_params))
    }
    fn parse_continuation(
        p: ProcessedResult<
            GetContinuationsQuery<'_, SearchQuery<'a, FilteredSearch<ArtistsFilter>>>,
        >,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let crawler: JsonCrawlerOwned = p.into();
        let (contents, continuation_params) =
            get_filtered_search_continuation_music_shelf_contents_and_params(crawler)?;
        let results = contents.try_into()?;
        Ok((results, continuation_params))
    }
}
impl<'a> ParseFromContinuable<SearchQuery<'a, FilteredSearch<ProfilesFilter>>>
    for Vec<SearchResultProfile>
{
    fn parse_from_continuable(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<ProfilesFilter>>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let mut section_contents = FilteredSearchSectionContents::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok((Vec::new(), None));
        }
        let continuation_params =
            take_continuation_params_from_section_contents(&mut section_contents)?;
        let results = FilteredSearchMusicShelfContents::try_from(section_contents)?.try_into()?;
        Ok((results, continuation_params))
    }
    fn parse_continuation(
        p: ProcessedResult<
            GetContinuationsQuery<'_, SearchQuery<'a, FilteredSearch<ProfilesFilter>>>,
        >,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let crawler: JsonCrawlerOwned = p.into();
        let (contents, continuation_params) =
            get_filtered_search_continuation_music_shelf_contents_and_params(crawler)?;
        let results = contents.try_into()?;
        Ok((results, continuation_params))
    }
}
impl<'a> ParseFromContinuable<SearchQuery<'a, FilteredSearch<AlbumsFilter>>>
    for Vec<SearchResultAlbum>
{
    fn parse_from_continuable(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<AlbumsFilter>>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let mut section_contents = FilteredSearchSectionContents::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok((Vec::new(), None));
        }
        let continuation_params =
            take_continuation_params_from_section_contents(&mut section_contents)?;
        let results = FilteredSearchMusicShelfContents::try_from(section_contents)?.try_into()?;
        Ok((results, continuation_params))
    }
    fn parse_continuation(
        p: ProcessedResult<
            GetContinuationsQuery<'_, SearchQuery<'a, FilteredSearch<AlbumsFilter>>>,
        >,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let crawler: JsonCrawlerOwned = p.into();
        let (contents, continuation_params) =
            get_filtered_search_continuation_music_shelf_contents_and_params(crawler)?;
        let results = contents.try_into()?;
        Ok((results, continuation_params))
    }
}
impl<'a> ParseFromContinuable<SearchQuery<'a, FilteredSearch<SongsFilter>>>
    for Vec<SearchResultSong>
{
    fn parse_from_continuable(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<SongsFilter>>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let mut section_contents = FilteredSearchSectionContents::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok((Vec::new(), None));
        }
        let continuation_params =
            take_continuation_params_from_section_contents(&mut section_contents)?;
        let results = FilteredSearchMusicShelfContents::try_from(section_contents)?.try_into()?;
        Ok((results, continuation_params))
    }
    fn parse_continuation(
        p: ProcessedResult<GetContinuationsQuery<'_, SearchQuery<'a, FilteredSearch<SongsFilter>>>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let crawler: JsonCrawlerOwned = p.into();
        let (contents, continuation_params) =
            get_filtered_search_continuation_music_shelf_contents_and_params(crawler)?;
        let results = contents.try_into()?;
        Ok((results, continuation_params))
    }
}
impl<'a> ParseFromContinuable<SearchQuery<'a, FilteredSearch<VideosFilter>>>
    for Vec<SearchResultVideo>
{
    fn parse_from_continuable(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<VideosFilter>>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let mut section_contents = FilteredSearchSectionContents::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok((Vec::new(), None));
        }
        let continuation_params =
            take_continuation_params_from_section_contents(&mut section_contents)?;
        let results = FilteredSearchMusicShelfContents::try_from(section_contents)?.try_into()?;
        Ok((results, continuation_params))
    }
    fn parse_continuation(
        p: ProcessedResult<
            GetContinuationsQuery<'_, SearchQuery<'a, FilteredSearch<VideosFilter>>>,
        >,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let crawler: JsonCrawlerOwned = p.into();
        let (contents, continuation_params) =
            get_filtered_search_continuation_music_shelf_contents_and_params(crawler)?;
        let results = contents.try_into()?;
        Ok((results, continuation_params))
    }
}
impl<'a> ParseFromContinuable<SearchQuery<'a, FilteredSearch<EpisodesFilter>>>
    for Vec<SearchResultEpisode>
{
    fn parse_from_continuable(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<EpisodesFilter>>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let mut section_contents = FilteredSearchSectionContents::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok((Vec::new(), None));
        }
        let continuation_params =
            take_continuation_params_from_section_contents(&mut section_contents)?;
        let results = FilteredSearchMusicShelfContents::try_from(section_contents)?.try_into()?;
        Ok((results, continuation_params))
    }
    fn parse_continuation(
        p: ProcessedResult<
            GetContinuationsQuery<'_, SearchQuery<'a, FilteredSearch<EpisodesFilter>>>,
        >,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let crawler: JsonCrawlerOwned = p.into();
        let (contents, continuation_params) =
            get_filtered_search_continuation_music_shelf_contents_and_params(crawler)?;
        let results = contents.try_into()?;
        Ok((results, continuation_params))
    }
}
impl<'a> ParseFromContinuable<SearchQuery<'a, FilteredSearch<PodcastsFilter>>>
    for Vec<SearchResultPodcast>
{
    fn parse_from_continuable(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<PodcastsFilter>>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let mut section_contents = FilteredSearchSectionContents::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok((Vec::new(), None));
        }
        let continuation_params =
            take_continuation_params_from_section_contents(&mut section_contents)?;
        let results = FilteredSearchMusicShelfContents::try_from(section_contents)?.try_into()?;
        Ok((results, continuation_params))
    }
    fn parse_continuation(
        p: ProcessedResult<
            GetContinuationsQuery<'_, SearchQuery<'a, FilteredSearch<PodcastsFilter>>>,
        >,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let crawler: JsonCrawlerOwned = p.into();
        let (contents, continuation_params) =
            get_filtered_search_continuation_music_shelf_contents_and_params(crawler)?;
        let results = contents.try_into()?;
        Ok((results, continuation_params))
    }
}
impl<'a> ParseFromContinuable<SearchQuery<'a, FilteredSearch<CommunityPlaylistsFilter>>>
    for Vec<SearchResultPlaylist>
{
    fn parse_from_continuable(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<CommunityPlaylistsFilter>>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let mut section_contents = FilteredSearchSectionContents::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok((Vec::new(), None));
        }
        let continuation_params =
            take_continuation_params_from_section_contents(&mut section_contents)?;
        let results = FilteredSearchMusicShelfContents::try_from(section_contents)?.try_into()?;
        Ok((results, continuation_params))
    }
    fn parse_continuation(
        p: ProcessedResult<
            GetContinuationsQuery<'_, SearchQuery<'a, FilteredSearch<CommunityPlaylistsFilter>>>,
        >,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let crawler: JsonCrawlerOwned = p.into();
        let (contents, continuation_params) =
            get_filtered_search_continuation_music_shelf_contents_and_params(crawler)?;
        let results = contents.try_into()?;
        Ok((results, continuation_params))
    }
}
impl<'a> ParseFromContinuable<SearchQuery<'a, FilteredSearch<FeaturedPlaylistsFilter>>>
    for Vec<SearchResultFeaturedPlaylist>
{
    fn parse_from_continuable(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<FeaturedPlaylistsFilter>>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let mut section_contents = FilteredSearchSectionContents::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok((Vec::new(), None));
        }
        let continuation_params =
            take_continuation_params_from_section_contents(&mut section_contents)?;
        let results = FilteredSearchMusicShelfContents::try_from(section_contents)?.try_into()?;
        Ok((results, continuation_params))
    }
    fn parse_continuation(
        p: ProcessedResult<
            GetContinuationsQuery<'_, SearchQuery<'a, FilteredSearch<FeaturedPlaylistsFilter>>>,
        >,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let crawler: JsonCrawlerOwned = p.into();
        let (contents, continuation_params) =
            get_filtered_search_continuation_music_shelf_contents_and_params(crawler)?;
        let results = contents.try_into()?;
        Ok((results, continuation_params))
    }
}
impl<'a> ParseFromContinuable<SearchQuery<'a, FilteredSearch<PlaylistsFilter>>>
    for Vec<SearchResultPlaylist>
{
    fn parse_from_continuable(
        p: ProcessedResult<SearchQuery<'a, FilteredSearch<PlaylistsFilter>>>,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let mut section_contents = FilteredSearchSectionContents::try_from(p)?;
        if section_contents_is_empty(&mut section_contents)? {
            return Ok((Vec::new(), None));
        }
        let continuation_params =
            take_continuation_params_from_section_contents(&mut section_contents)?;
        let results = FilteredSearchMusicShelfContents::try_from(section_contents)?.try_into()?;
        Ok((results, continuation_params))
    }
    fn parse_continuation(
        p: ProcessedResult<
            GetContinuationsQuery<'_, SearchQuery<'a, FilteredSearch<PlaylistsFilter>>>,
        >,
    ) -> crate::Result<(Self, Option<crate::common::ContinuationParams<'static>>)> {
        let crawler: JsonCrawlerOwned = p.into();
        let (contents, continuation_params) =
            get_filtered_search_continuation_music_shelf_contents_and_params(crawler)?;
        let results = contents.try_into()?;
        Ok((results, continuation_params))
    }
}

impl<'a> ParseFrom<GetSearchSuggestionsQuery<'a>> for Vec<SearchSuggestion> {
    fn parse_from(p: ProcessedResult<GetSearchSuggestionsQuery<'a>>) -> crate::Result<Self> {
        let json_crawler: JsonCrawlerOwned = p.into();
        let mut suggestions = json_crawler
            .navigate_pointer("/contents/0/searchSuggestionsSectionRenderer/contents")?;
        let mut results = Vec::new();
        for mut s in suggestions.try_iter_mut()? {
            let mut runs = Vec::new();
            if let Ok(mut search_suggestion) =
                s.borrow_pointer("/searchSuggestionRenderer/suggestion/runs")
            {
                for mut r in search_suggestion.try_iter_mut()? {
                    if let Ok(true) = r.take_value_pointer("/bold") {
                        runs.push(r.take_value_pointer("/text").map(TextRun::Bold)?)
                    } else {
                        runs.push(r.take_value_pointer("/text").map(TextRun::Normal)?)
                    }
                }
                results.push(SearchSuggestion::new(SuggestionType::Prediction, runs))
            } else {
                for mut r in s
                    .borrow_pointer("/historySuggestionRenderer/suggestion/runs")?
                    .try_iter_mut()?
                {
                    if let Ok(true) = r.take_value_pointer("/bold") {
                        runs.push(r.take_value_pointer("/text").map(TextRun::Bold)?)
                    } else {
                        runs.push(r.take_value_pointer("/text").map(TextRun::Normal)?)
                    }
                }
                results.push(SearchSuggestion::new(SuggestionType::History, runs))
            }
        }
        Ok(results)
    }
}

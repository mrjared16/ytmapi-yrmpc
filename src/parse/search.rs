use super::{DISPLAY_POLICY, ParseFrom, ProcessedResult, flex_column_item_pointer};
use crate::common::{
    AlbumID, AlbumType, ArtistChannelID, ContinuationParams, EpisodeID, Explicit, PlaylistID,
    PodcastID, SearchSuggestion, SuggestionType, TextRun, Thumbnail, UserChannelID, VideoID,
    YoutubeID,
};
use crate::continuations::ParseFromContinuable;
use crate::nav_consts::{
    BADGE_LABEL, CONTINUATION_PARAMS, LIVE_BADGE_LABEL, MRLIR, MUSIC_CARD_SHELF, MUSIC_SHELF,
    MUSIC_SHELF_CONTINUATION, NAVIGATION_BROWSE, NAVIGATION_BROWSE_ID, NAVIGATION_VIDEO_ID,
    ON_TAP_VIDEO_ID, PAGE_TYPE, PLAY_BUTTON, PLAYLIST_ITEM_VIDEO_ID, SECTION_LIST, SUBTITLE2,
    TAB_CONTENT, THUMBNAILS, TITLE_NAV_VIDEO_ID, TITLE_TEXT, WATCH_VIDEO_ID,
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum SearchRowSource {
    MusicShelfRow,
    CardPrimary,
    CardChild,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SearchTextRun {
    text: String,
    browse_id: Option<String>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct SearchTextSegment {
    text: String,
    browse_ids: Vec<String>,
}

impl SearchTextSegment {
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

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct SearchTextLine {
    text: String,
    runs: Vec<SearchTextRun>,
    bullet_segments: Vec<SearchTextSegment>,
}

impl SearchTextLine {
    fn first_segment_text(&self) -> Option<String> {
        self.bullet_segments
            .first()
            .map(SearchTextSegment::trimmed_text)
    }

    fn raw_segment_text(&self, idx: usize) -> Option<String> {
        self.bullet_segments
            .get(idx)
            .map(|segment| segment.text.clone())
    }

    fn first_browse_id_with_prefix(&self, prefixes: &[&str]) -> Option<String> {
        self.bullet_segments
            .iter()
            .find_map(|segment| segment.first_browse_id_with_prefix(prefixes))
            .or_else(|| {
                self.runs.iter().find_map(|run| {
                    run.browse_id.as_ref().and_then(|browse_id| {
                        (prefixes.is_empty()
                            || prefixes.iter().any(|prefix| browse_id.starts_with(prefix)))
                        .then(|| browse_id.clone())
                    })
                })
            })
    }

    fn detail_segments<'a>(&'a self, labels: &[&str]) -> Vec<&'a SearchTextSegment> {
        self.bullet_segments
            .iter()
            .filter(|segment| {
                let text = segment.trimmed_text();
                !text.is_empty() && !labels.iter().any(|label| text.eq_ignore_ascii_case(label))
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
struct SearchRow {
    source: SearchRowSource,
    title: Option<SearchTextLine>,
    metadata_lines: Vec<SearchTextLine>,
    thumbnails: Vec<Thumbnail>,
    explicit: bool,
    live: bool,
    display_policy: Option<String>,
    root_browse_id: Option<String>,
    root_page_type: Option<YoutubeMusicPageType>,
    video_id: Option<String>,
}

impl SearchRow {
    fn title_text(&self) -> String {
        self.title
            .as_ref()
            .map(|line| line.text.trim().to_string())
            .unwrap_or_default()
    }

    fn first_metadata_line(&self) -> Option<&SearchTextLine> {
        self.metadata_lines.first()
    }

    fn metadata_line(&self, idx: usize) -> Option<&SearchTextLine> {
        self.metadata_lines.get(idx)
    }

    fn browse_id_with_prefix(&self, prefixes: &[&str]) -> Option<String> {
        self.root_browse_id
            .as_ref()
            .and_then(|browse_id| {
                (prefixes.is_empty() || prefixes.iter().any(|prefix| browse_id.starts_with(prefix)))
                    .then(|| browse_id.clone())
            })
            .or_else(|| {
                self.title
                    .as_ref()
                    .and_then(|line| line.first_browse_id_with_prefix(prefixes))
            })
            .or_else(|| {
                self.metadata_lines
                    .iter()
                    .find_map(|line| line.first_browse_id_with_prefix(prefixes))
            })
    }
}

fn split_bullet_segments(runs: Vec<SearchTextRun>) -> SearchTextLine {
    let text = runs.iter().map(|run| run.text.as_str()).collect::<String>();
    let mut bullet_segments = Vec::new();
    let mut current = SearchTextSegment::default();

    for run in &runs {
        if run.text == " • " {
            if !current.text.trim().is_empty() {
                bullet_segments.push(current);
                current = SearchTextSegment::default();
            }
            continue;
        }

        current.text.push_str(&run.text);
        if let Some(browse_id) = &run.browse_id {
            current.browse_ids.push(browse_id.clone());
        }
    }

    if !current.text.trim().is_empty() {
        bullet_segments.push(current);
    }

    SearchTextLine {
        text,
        runs,
        bullet_segments,
    }
}

fn parse_text_line(
    item: &mut impl JsonCrawler,
    pointer: impl AsRef<str>,
) -> Result<SearchTextLine> {
    let mut runs = item.borrow_pointer(format!("{}/runs", pointer.as_ref()))?;
    let runs = runs
        .try_iter_mut()?
        .filter_map(|run| {
            let text = run.borrow_value_pointer::<String>("/text").ok()?;
            Some(SearchTextRun {
                browse_id: run
                    .borrow_value_pointer::<String>(NAVIGATION_BROWSE_ID)
                    .ok(),
                text,
            })
        })
        .collect();
    Ok(split_bullet_segments(runs))
}

fn parse_music_shelf_row(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
    source: SearchRowSource,
) -> Result<SearchRow> {
    let mut mrlir = music_shelf_contents.navigate_pointer(MRLIR)?;
    let title = parse_text_line(&mut mrlir, format!("{}/text", flex_column_item_pointer(0))).ok();
    let metadata_lines = (1..4)
        .filter_map(|col_idx| {
            parse_text_line(
                &mut mrlir,
                format!("{}/text", flex_column_item_pointer(col_idx)),
            )
            .ok()
        })
        .collect_vec();
    let display_policy = mrlir.take_value_pointer(DISPLAY_POLICY).ok();
    let root_page_type = mrlir
        .borrow_value_pointer(concatcp!(NAVIGATION_BROWSE, PAGE_TYPE))
        .ok();
    let root_browse_id = mrlir.take_value_pointer(NAVIGATION_BROWSE_ID).ok();
    let video_id = mrlir
        .take_value_pointer(PLAYLIST_ITEM_VIDEO_ID)
        .ok()
        .or_else(|| mrlir.take_value_pointer(TITLE_NAV_VIDEO_ID).ok())
        .or_else(|| mrlir.take_value_pointer(ON_TAP_VIDEO_ID).ok())
        .or_else(|| mrlir.take_value_pointer(NAVIGATION_VIDEO_ID).ok())
        .or_else(|| {
            mrlir
                .take_value_pointer(concatcp!(PLAY_BUTTON, WATCH_VIDEO_ID))
                .ok()
        });
    let thumbnails = mrlir
        .take_value_pointer(THUMBNAILS)
        .ok()
        .unwrap_or_default();
    Ok(SearchRow {
        source,
        title,
        metadata_lines,
        thumbnails,
        explicit: mrlir.path_exists(BADGE_LABEL),
        live: mrlir.path_exists(LIVE_BADGE_LABEL),
        display_policy,
        root_browse_id,
        root_page_type,
        video_id,
    })
}

fn parse_card_primary_row(mut card: JsonCrawlerBorrowed<'_>) -> Result<SearchRow> {
    let title = parse_text_line(&mut card, "/title").ok();
    let metadata_lines = ["/subtitle", SUBTITLE2]
        .into_iter()
        .filter_map(|pointer| parse_text_line(&mut card, pointer).ok())
        .collect_vec();
    let root_page_type = card
        .borrow_value_pointer(concatcp!(NAVIGATION_BROWSE, PAGE_TYPE))
        .ok();
    let root_browse_id = card.take_value_pointer(NAVIGATION_BROWSE_ID).ok();
    let video_id = card
        .take_value_pointer(ON_TAP_VIDEO_ID)
        .ok()
        .or_else(|| card.take_value_pointer(TITLE_NAV_VIDEO_ID).ok())
        .or_else(|| card.take_value_pointer(PLAYLIST_ITEM_VIDEO_ID).ok())
        .or_else(|| card.take_value_pointer(NAVIGATION_VIDEO_ID).ok())
        .or_else(|| {
            card.take_value_pointer(concatcp!(PLAY_BUTTON, WATCH_VIDEO_ID))
                .ok()
        });
    let thumbnails = card.take_value_pointer(THUMBNAILS).ok().unwrap_or_default();

    Ok(SearchRow {
        source: SearchRowSource::CardPrimary,
        title,
        metadata_lines,
        thumbnails,
        explicit: false,
        live: false,
        display_policy: None,
        root_browse_id,
        root_page_type,
        video_id,
    })
}

fn is_duration_text(text: &str) -> bool {
    let parts = text.trim().split(':').collect_vec();
    (2..=3).contains(&parts.len())
        && parts
            .iter()
            .all(|part| !part.is_empty() && part.chars().all(|ch| ch.is_ascii_digit()))
}

fn is_views_like_text(text: &str) -> bool {
    text.trim().to_ascii_lowercase().contains("views")
}

fn is_episode_date_like(text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }
    if text.to_ascii_lowercase().contains("ago") {
        return true;
    }
    const MONTH_PREFIXES: &[&str] = &[
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    text.split_whitespace()
        .next()
        .map(|part| {
            MONTH_PREFIXES
                .iter()
                .any(|prefix| part.to_ascii_lowercase().starts_with(prefix))
        })
        .unwrap_or(false)
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

fn join_artist_segments(segments: &[&SearchTextSegment]) -> String {
    segments
        .iter()
        .map(|segment| segment.trimmed_text())
        .filter(|text| !text.is_empty())
        .join(" & ")
}

fn parse_song_metadata_line(
    line: Option<&SearchTextLine>,
) -> (String, Option<ParsedSongAlbum>, String) {
    let Some(line) = line else {
        return (String::new(), None, String::new());
    };

    let filtered_segments = line
        .bullet_segments
        .iter()
        .filter(|segment| !is_song_metadata_label(&segment.text))
        .collect_vec();

    let duration_idx = filtered_segments
        .iter()
        .rposition(|segment| is_duration_text(&segment.text));
    let duration = duration_idx
        .and_then(|idx| filtered_segments.get(idx))
        .map(|segment| segment.trimmed_text())
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

    (artist, album, duration)
}

fn parse_untyped_top_result_metadata_line(
    line: Option<&SearchTextLine>,
) -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let Some(line) = line else {
        return (None, None, None, None);
    };

    let segments = line.bullet_segments.iter().collect_vec();
    let duration_idx = segments
        .iter()
        .rposition(|segment| is_duration_text(&segment.text));
    let duration = duration_idx
        .and_then(|idx| segments.get(idx))
        .map(|segment| segment.trimmed_text())
        .filter(|text| !text.is_empty());

    let metadata_segments = segments
        .into_iter()
        .enumerate()
        .filter_map(|(idx, segment)| (Some(idx) != duration_idx).then_some(segment))
        .collect_vec();

    if metadata_segments.is_empty() {
        return (None, None, duration, None);
    }

    let album_idx = metadata_segments
        .iter()
        .rposition(|segment| segment.first_browse_id_with_prefix(&["MP"]).is_some());

    if let Some(idx) = album_idx.filter(|idx| *idx > 0) {
        let artist = join_artist_segments(&metadata_segments[..idx]);
        let album = metadata_segments[idx].trimmed_text();
        return (
            (!artist.is_empty()).then_some(artist),
            (!album.is_empty()).then_some(album),
            duration,
            None,
        );
    }

    let artist = metadata_segments
        .first()
        .map(|segment| segment.trimmed_text())
        .filter(|text| !text.is_empty());
    let secondary = metadata_segments
        .get(1)
        .map(|segment| segment.trimmed_text())
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

    (artist, album, duration, plays)
}

fn detail_values(line: Option<&SearchTextLine>, labels: &[&str]) -> Vec<String> {
    line.map(|line| {
        line.detail_segments(labels)
            .into_iter()
            .map(SearchTextSegment::trimmed_text)
            .filter(|text| !text.is_empty())
            .collect_vec()
    })
    .unwrap_or_default()
}

fn parse_album_metadata_line(line: Option<&SearchTextLine>) -> (AlbumType, String, String) {
    let Some(line) = line else {
        return (
            AlbumType::Album,
            "Unknown Artist".to_string(),
            "Unknown Year".to_string(),
        );
    };

    let album_type = line
        .first_segment_text()
        .and_then(|text| parse_album_type_label(&text))
        .unwrap_or(AlbumType::Album);
    let detail_segments = line.detail_segments(&["Album", "Single", "EP"]);
    let artist = detail_segments
        .first()
        .map(|segment| segment.trimmed_text())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "Unknown Artist".to_string());
    let year = detail_segments
        .get(1)
        .map(|segment| segment.trimmed_text())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "Unknown Year".to_string());
    (album_type, artist, year)
}

fn parse_top_result_type(row: &SearchRow) -> Option<TopResultType> {
    let first = row
        .first_metadata_line()
        .and_then(SearchTextLine::first_segment_text)?;
    let parsed = TopResultType::from_subtitle(&first);
    match (&row.source, parsed) {
        (SearchRowSource::CardPrimary, parsed) => Some(parsed),
        (_, TopResultType::Unknown(_)) => None,
        (_, parsed) => Some(parsed),
    }
}

fn parse_top_result_from_row(row: SearchRow) -> TopResult {
    if row.source == SearchRowSource::CardPrimary {
        let result_type = parse_top_result_type(&row)
            .or_else(|| Some(TopResultType::Unknown("missing".to_string())));
        let secondary = row
            .metadata_line(1)
            .map(|line| line.text.trim().to_string());
        let secondary_first_segment = row
            .metadata_line(1)
            .and_then(SearchTextLine::first_segment_text);
        let primary_details = detail_values(row.first_metadata_line(), &[]);
        let browse_id = match result_type.as_ref() {
            Some(TopResultType::Artist) => row
                .root_browse_id
                .clone()
                .or_else(|| row.browse_id_with_prefix(&["UC"])),
            Some(TopResultType::Album(_)) => row
                .root_browse_id
                .clone()
                .or_else(|| row.browse_id_with_prefix(&["MP"])),
            Some(TopResultType::Playlist | TopResultType::Station) => row
                .root_browse_id
                .clone()
                .or_else(|| row.browse_id_with_prefix(&["VL", "PL", "RD"])),
            Some(TopResultType::Unknown(_)) if row.video_id.is_none() => row
                .root_browse_id
                .clone()
                .or_else(|| row.browse_id_with_prefix(&["VL", "PL", "RD"])),
            _ => row.root_browse_id.clone(),
        };
        let (artist, subscribers) = match result_type.as_ref() {
            Some(TopResultType::Artist) => (
                None,
                detail_values(row.first_metadata_line(), &["Artist"])
                    .into_iter()
                    .next()
                    .or(secondary),
            ),
            Some(TopResultType::Unknown(_)) if row.video_id.is_some() => (
                secondary_first_segment.or_else(|| primary_details.get(1).cloned()),
                None,
            ),
            _ if row.video_id.is_some() => (secondary, None),
            _ => (None, secondary),
        };
        let byline = match result_type.as_ref() {
            Some(TopResultType::Unknown(_)) if row.video_id.is_some() => {
                primary_details.first().cloned()
            }
            Some(TopResultType::Unknown(_)) => row
                .first_metadata_line()
                .map(|line| line.text.trim().to_string()),
            _ => None,
        };

        return TopResult {
            result_name: row.title_text(),
            result_type,
            thumbnails: row.thumbnails,
            artist,
            album: None,
            duration: None,
            year: None,
            subscribers,
            plays: None,
            publisher: None,
            byline,
            browse_id,
            video_id: row.video_id,
        };
    }

    let result_name = row.title_text();
    let result_type = parse_top_result_type(&row);
    let mut subscribers = None;
    let mut publisher = None;
    let mut artist = None;
    let mut album = None;
    let mut duration = None;
    let mut year = None;
    let mut plays = None;
    let byline = None;

    match result_type.as_ref() {
        Some(TopResultType::Artist) => {
            subscribers = detail_values(row.first_metadata_line(), &["Artist"])
                .into_iter()
                .next();
        }
        Some(TopResultType::Album(_)) => {
            let (_, parsed_artist, parsed_year) =
                parse_album_metadata_line(row.first_metadata_line());
            artist = Some(parsed_artist);
            year = Some(parsed_year);
        }
        Some(TopResultType::Playlist) => {
            artist = detail_values(row.first_metadata_line(), &["Playlist"])
                .into_iter()
                .next();
        }
        Some(TopResultType::Song) => {
            let (parsed_artist, parsed_album, parsed_duration) =
                parse_song_metadata_line(row.first_metadata_line());
            artist = (!parsed_artist.is_empty()).then_some(parsed_artist);
            album = parsed_album.map(|album| album.name);
            duration = (!parsed_duration.is_empty()).then_some(parsed_duration);
            plays = row
                .metadata_line(1)
                .map(|line| line.text.trim().to_string());
        }
        Some(TopResultType::Video) => {
            let details = detail_values(row.first_metadata_line(), &["Video"]);
            artist = details.first().cloned();
            duration = details.get(2).cloned().or_else(|| details.get(1).cloned());
            plays = details
                .get(1)
                .cloned()
                .filter(|text| text.contains("views"));
        }
        Some(TopResultType::Station) => {
            subscribers = detail_values(row.first_metadata_line(), &["Station"])
                .into_iter()
                .next();
        }
        Some(TopResultType::Podcast) => {
            publisher = detail_values(row.first_metadata_line(), &["Podcast"])
                .into_iter()
                .next();
        }
        Some(TopResultType::Unknown(_)) | None => {
            let (parsed_artist, parsed_album, parsed_duration, parsed_plays) =
                parse_untyped_top_result_metadata_line(row.first_metadata_line());
            artist = parsed_artist.or_else(|| {
                row.first_metadata_line()
                    .and_then(|line| line.first_segment_text())
            });
            album = parsed_album;
            duration = parsed_duration;
            plays = parsed_plays.or_else(|| {
                row.metadata_line(1)
                    .map(|line| line.text.trim().to_string())
            });
        }
    }

    let browse_id = match result_type.as_ref() {
        Some(TopResultType::Artist) => row.browse_id_with_prefix(&["UC"]),
        Some(TopResultType::Album(_)) => row.browse_id_with_prefix(&["MP"]),
        Some(TopResultType::Playlist) | Some(TopResultType::Station) => {
            row.browse_id_with_prefix(&["VL", "PL", "RD"])
        }
        _ => row.root_browse_id.clone(),
    };

    TopResult {
        result_name,
        result_type,
        thumbnails: row.thumbnails,
        artist,
        album,
        duration,
        year,
        subscribers,
        plays,
        publisher,
        byline,
        browse_id,
        video_id: row.video_id,
    }
}

fn parse_artist_result_from_row(row: SearchRow) -> Result<SearchResultArtist> {
    let browse_id = row
        .browse_id_with_prefix(&["UC"])
        .map(ArtistChannelID::from_raw)
        .ok_or_else(|| {
            Error::other_code(0, "Artist search result missing browse id".to_string())
        })?;
    let subscribers = detail_values(row.first_metadata_line(), &["Artist"])
        .into_iter()
        .next();
    Ok(SearchResultArtist {
        artist: row.title_text(),
        subscribers,
        browse_id,
        thumbnails: row.thumbnails,
    })
}

fn parse_profile_result_from_row(row: SearchRow) -> Result<SearchResultProfile> {
    let profile_id = row.browse_id_with_prefix(&[]).ok_or_else(|| {
        Error::other_code(0, "Profile search result missing browse id".to_string())
    })?;
    Ok(SearchResultProfile {
        title: row.title_text(),
        username: detail_values(row.first_metadata_line(), &["Profile"])
            .into_iter()
            .next()
            .unwrap_or_default(),
        profile_id: UserChannelID::from_raw(profile_id),
        thumbnails: row.thumbnails,
    })
}

fn parse_album_result_from_row(row: SearchRow) -> Result<SearchResultAlbum> {
    let album_id = row
        .browse_id_with_prefix(&["MP"])
        .map(AlbumID::from_raw)
        .ok_or_else(|| Error::other_code(0, "Album search result missing browse id".to_string()))?;
    let (album_type, artist, year) = parse_album_metadata_line(row.first_metadata_line());
    Ok(SearchResultAlbum {
        title: row.title_text(),
        artist,
        year,
        explicit: if row.explicit {
            Explicit::IsExplicit
        } else {
            Explicit::NotExplicit
        },
        album_id,
        album_type,
        thumbnails: row.thumbnails,
    })
}

fn parse_song_result_from_row(row: SearchRow) -> Result<SearchResultSong> {
    let (artist, album, duration) = parse_song_metadata_line(row.first_metadata_line());
    let video_id = row
        .video_id
        .clone()
        .ok_or_else(|| Error::other_code(0, "Song search result missing video id".to_string()))?;
    Ok(SearchResultSong {
        title: row.title_text(),
        artist,
        album,
        duration,
        plays: row
            .metadata_line(1)
            .map(|line| line.text.trim().to_string())
            .unwrap_or_default(),
        explicit: if row.explicit {
            Explicit::IsExplicit
        } else {
            Explicit::NotExplicit
        },
        video_id: VideoID::from_raw(video_id),
        thumbnails: row.thumbnails,
    })
}

fn parse_video_result_from_row(row: SearchRow) -> Result<Option<SearchResultVideo>> {
    if row.display_policy.as_deref() == Some("MUSIC_ITEM_RENDERER_DISPLAY_POLICY_GREY_OUT") {
        return Ok(None);
    }

    let title = row.title_text();
    let details = detail_values(row.first_metadata_line(), &["Video", "Episode"]);
    let first_segment = row
        .first_metadata_line()
        .and_then(SearchTextLine::first_segment_text)
        .unwrap_or_default();
    let first_segment_raw = row
        .first_metadata_line()
        .and_then(|line| line.bullet_segments.first())
        .map(|segment| segment.text.clone())
        .unwrap_or_default();
    let video_id = row
        .video_id
        .clone()
        .ok_or_else(|| Error::other_code(0, "Video search result missing video id".to_string()))?;

    match first_segment.as_str() {
        "Episode" => Ok(Some(SearchResultVideo::VideoEpisode {
            title,
            date: if row.live {
                EpisodeDate::Live
            } else {
                EpisodeDate::Recorded {
                    date: details.first().cloned().unwrap_or_default(),
                }
            },
            channel_name: details.get(1).cloned().unwrap_or_default(),
            episode_id: EpisodeID::from_raw(video_id),
            thumbnails: row.thumbnails,
        })),
        "Video" => Ok(Some(SearchResultVideo::Video {
            title,
            channel_name: details.first().cloned().unwrap_or_default(),
            views: details.get(1).cloned().unwrap_or_default(),
            length: details.get(2).cloned().unwrap_or_default(),
            video_id: VideoID::from_raw(video_id),
            thumbnails: row.thumbnails,
        })),
        _ if row.root_page_type == Some(YoutubeMusicPageType::Podcast) => {
            Ok(Some(SearchResultVideo::VideoEpisode {
                title,
                date: if row.live {
                    EpisodeDate::Live
                } else {
                    EpisodeDate::Recorded {
                        date: details.first().cloned().unwrap_or_default(),
                    }
                },
                channel_name: details.get(1).cloned().unwrap_or_default(),
                episode_id: EpisodeID::from_raw(video_id),
                thumbnails: row.thumbnails,
            }))
        }
        _ if is_episode_date_like(&first_segment)
            && details.get(1).is_some()
            && !is_views_like_text(&first_segment) =>
        {
            Ok(Some(SearchResultVideo::VideoEpisode {
                title,
                date: if row.live {
                    EpisodeDate::Live
                } else {
                    EpisodeDate::Recorded {
                        date: first_segment,
                    }
                },
                channel_name: details.get(1).cloned().unwrap_or_default(),
                episode_id: EpisodeID::from_raw(video_id),
                thumbnails: row.thumbnails,
            }))
        }
        _ => Ok(Some(SearchResultVideo::Video {
            title,
            channel_name: first_segment_raw,
            views: details.get(1).cloned().unwrap_or_default(),
            length: details.get(2).cloned().unwrap_or_default(),
            video_id: VideoID::from_raw(video_id),
            thumbnails: row.thumbnails,
        })),
    }
}

fn parse_podcast_result_from_row(row: SearchRow) -> Result<SearchResultPodcast> {
    let podcast_id = row.browse_id_with_prefix(&[]).ok_or_else(|| {
        Error::other_code(0, "Podcast search result missing browse id".to_string())
    })?;
    Ok(SearchResultPodcast {
        title: row.title_text(),
        publisher: row
            .first_metadata_line()
            .and_then(|line| line.raw_segment_text(0))
            .unwrap_or_default(),
        podcast_id: PodcastID::from_raw(podcast_id),
        thumbnails: row.thumbnails,
    })
}

fn parse_episode_result_from_row(row: SearchRow) -> Result<SearchResultEpisode> {
    let first_line = row.first_metadata_line();
    let episode_id = row.video_id.clone().ok_or_else(|| {
        Error::other_code(0, "Episode search result missing video id".to_string())
    })?;
    let date = if row.live {
        EpisodeDate::Live
    } else {
        EpisodeDate::Recorded {
            date: first_line
                .and_then(|line| line.raw_segment_text(0))
                .unwrap_or_default(),
        }
    };
    let channel_name = if row.live {
        first_line
            .and_then(|line| line.raw_segment_text(0))
            .unwrap_or_default()
    } else {
        first_line
            .and_then(|line| line.raw_segment_text(1))
            .unwrap_or_default()
    };

    Ok(SearchResultEpisode {
        title: row.title_text(),
        date,
        channel_name,
        episode_id: EpisodeID::from_raw(episode_id),
        thumbnails: row.thumbnails,
    })
}

fn parse_featured_playlist_result_from_row(row: SearchRow) -> Result<SearchResultFeaturedPlaylist> {
    let playlist_id = row
        .browse_id_with_prefix(&["VL", "PL", "RD"])
        .map(PlaylistID::from_raw)
        .ok_or_else(|| {
            Error::other_code(
                0,
                "Featured playlist search result missing browse id".to_string(),
            )
        })?;
    let details = detail_values(row.first_metadata_line(), &["Playlist"]);
    Ok(SearchResultFeaturedPlaylist {
        title: row.title_text(),
        author: details.first().cloned().unwrap_or_default(),
        songs: details.get(1).cloned().unwrap_or_default(),
        playlist_id,
        thumbnails: row.thumbnails,
    })
}

fn parse_community_playlist_result_from_row(
    row: SearchRow,
) -> Result<SearchResultCommunityPlaylist> {
    let playlist_id = row
        .browse_id_with_prefix(&["VL", "PL", "RD"])
        .map(PlaylistID::from_raw)
        .ok_or_else(|| {
            Error::other_code(
                0,
                "Community playlist search result missing browse id".to_string(),
            )
        })?;
    let details = detail_values(row.first_metadata_line(), &["Playlist"]);
    Ok(SearchResultCommunityPlaylist {
        title: row.title_text(),
        author: details.first().cloned().unwrap_or_default(),
        views: details.get(1).cloned().unwrap_or_default(),
        playlist_id,
        thumbnails: row.thumbnails,
    })
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
    results.push(parse_top_result_from_row(parse_card_primary_row(
        music_shelf_contents.borrow_pointer("")?,
    )?));
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
    Ok(Some(parse_top_result_from_row(parse_music_shelf_row(
        music_shelf_contents,
        SearchRowSource::CardChild,
    )?)))
}
// TODO: Type safety
// TODO: Tests
fn parse_artist_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultArtist> {
    parse_artist_result_from_row(parse_music_shelf_row(
        music_shelf_contents,
        SearchRowSource::MusicShelfRow,
    )?)
}

// TODO: Type safety
// TODO: Tests
fn parse_profile_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultProfile> {
    parse_profile_result_from_row(parse_music_shelf_row(
        music_shelf_contents,
        SearchRowSource::MusicShelfRow,
    )?)
}

// TODO: Type safety
// TODO: Tests
fn parse_album_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultAlbum> {
    parse_album_result_from_row(parse_music_shelf_row(
        music_shelf_contents,
        SearchRowSource::MusicShelfRow,
    )?)
}
fn parse_song_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultSong> {
    parse_song_result_from_row(parse_music_shelf_row(
        music_shelf_contents,
        SearchRowSource::MusicShelfRow,
    )?)
}
// TODO: Type safety
// TODO: Tests
fn parse_video_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<Option<SearchResultVideo>> {
    parse_video_result_from_row(parse_music_shelf_row(
        music_shelf_contents,
        SearchRowSource::MusicShelfRow,
    )?)
}
// TODO: Type safety
// TODO: Tests
fn parse_podcast_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultPodcast> {
    parse_podcast_result_from_row(parse_music_shelf_row(
        music_shelf_contents,
        SearchRowSource::MusicShelfRow,
    )?)
}
// TODO: Type safety
// TODO: Tests
fn parse_episode_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultEpisode> {
    parse_episode_result_from_row(parse_music_shelf_row(
        music_shelf_contents,
        SearchRowSource::MusicShelfRow,
    )?)
}
// TODO: Type safety
// TODO: Tests
fn parse_featured_playlist_search_result_from_music_shelf_contents(
    music_shelf_contents: JsonCrawlerBorrowed<'_>,
) -> Result<SearchResultFeaturedPlaylist> {
    parse_featured_playlist_result_from_row(parse_music_shelf_row(
        music_shelf_contents,
        SearchRowSource::MusicShelfRow,
    )?)
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
    parse_community_playlist_result_from_row(parse_music_shelf_row(
        music_shelf_contents,
        SearchRowSource::MusicShelfRow,
    )?)
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

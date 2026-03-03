use crate::models::card::{Source, Platform, Confidence};
use crate::api::reddit::RedditPost;

pub struct AuthenticityScorer;

impl AuthenticityScorer {
    /// Score a Reddit post 0.0-1.0 for likelihood of being real
    pub fn score_reddit(post: &RedditPost, trusted_users: &[String]) -> f32 {
        let mut score = 0.0;

        // Source reputation (40% max)
        if trusted_users.contains(&post.author) {
            score += 0.40;
        } else if post.author.to_lowercase().contains("wotc")
               || post.author.to_lowercase().contains("wizard") {
            score += 0.35;
        }

        // Content signals (50% max)
        // These would be filled by image analysis in production
        score += Self::score_title_patterns(&post.title);

        // Community validation (20% max, capped)
        let upvote_score = (post.upvotes as f32 / 100.0).min(0.2);
        score += upvote_score;

        // Negative signals
        if let Some(ref flair) = post.flair {
            let f = flair.to_lowercase();
            if f.contains("meme") || f.contains("shitpost") || f.contains("joke") {
                score -= 0.50;
            }
            if f.contains("speculation") || f.contains("theory") {
                score -= 0.30;
            }
        }

        // Self posts with no images are suspicious for spoilers
        if post.is_self && post.selftext.len() < 50 {
            score -= 0.20;
        }

        score.max(0.0).min(1.0)
    }

    fn score_title_patterns(title: &str) -> f32 {
        let mut score = 0.0;
        let t = title.to_lowercase();

        // Good signals
        if regex::Regex::new(r"\[\w{2,4}\]").unwrap().is_match(&t) {
            score += 0.15; // Has set code format
        }
        if t.contains("(") && t.contains(")") {
            score += 0.10; // Likely has mana cost
        }
        if t.contains("|") || t.contains("—") {
            score += 0.10; // Type line separator
        }

        // Bad signals
        if t.contains("??") || t.contains("unknown") || t.contains("leak?") {
            score -= 0.15; // Uncertainty markers
        }
        if t.contains("fake") || t.contains("not real") {
            score -= 0.50;
        }

        score.max(0.0).min(0.50)
    }

    /// Determine confidence level based on sources and scores
    pub fn determine_confidence(sources: &[Source]) -> Confidence {
        let has_official = sources.iter().any(|s| matches!(s.platform, Platform::WotC | Platform::Scryfall));
        let has_reddit = sources.iter().any(|s| matches!(s.platform, Platform::Reddit));
        let has_mythic = sources.iter().any(|s| matches!(s.platform, Platform::MythicSpoiler));

        let avg_score: f32 = sources.iter().map(|s| s.authenticity_score).sum::<f32>()
            / sources.len() as f32;

        match (has_official, has_mythic, has_reddit, avg_score) {
            (true, _, _, _) => Confidence::Official,
            (_, true, _, s) if s > 0.6 => Confidence::Confirmed,
            (_, _, true, s) if s > 0.7 => Confidence::Probable,
            (_, _, true, _) => Confidence::Rumor,
            _ => Confidence::Rumor,
        }
    }
}

// AuthenticityScorer is retained as a stub so fetcher.rs compiles without changes to its
// import. Heuristic scoring has been removed — confidence is now binary (Verified/Unverified)
// determined by Scryfall lookup only.
pub struct AuthenticityScorer;

impl AuthenticityScorer {
    /// Returns a fixed score of 0.5 for all Reddit posts.
    /// Authenticity scoring has been replaced by binary Scryfall verification.
    pub fn score_reddit(_post: &crate::api::reddit::RedditPost, _trusted_users: &[String]) -> f32 {
        0.5
    }
}

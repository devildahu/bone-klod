//! Game states.
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub enum GameState {
    MainMenu,
    Pause,
    #[cfg(feature = "editor")]
    Editor,
    /// The game is running
    Playing,
    TimeUp,
    /// Restart menu after gameover
    GameComplete,
}

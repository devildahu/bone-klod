//! Game states.
#[derive(Clone, Copy, Hash, PartialEq, Eq, Debug)]
pub enum GameState {
    MainMenu,
    /// Wait until the game scene is fully loaded if not already
    WaitLoaded,
    /// The game is running
    Playing,
    /// Restart menu after gameover
    RestartMenu,
}

//! Esito di un evento in una schermata: passa a un'altra, esce, o resta.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next<S> {
    Go(S),
    Quit,
    Stay,
}

//#[derive(Clone)]
//pub struct WorldState {}

#[derive(Clone, PartialEq, Copy)]
pub enum Dimensions {
    Overworld,
    Nether,
    End,
    Custom, // assumes overworld height
}

impl Dimensions {
    pub const fn get_y_bounds(self: Self) -> (i16, i16) {
        match self {
            Dimensions::Nether => (0, 255), // worldgen limit is 128, but players can go above that
            Dimensions::End => (0, 255),
            Dimensions::Overworld => (-64, 320),
            Dimensions::Custom => (-64, 320),
        }
    }
}

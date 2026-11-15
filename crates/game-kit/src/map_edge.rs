//! Map edges: when the legend says `".": edge`, each `.` in the grid is an edge cell (`dev-docs/extensions/map-files.md`, "Map edges").
//! The parser finds the edge cells (`map::parse`) and gives each one a piece with `choose_pieces`. A game chooses an
//! `EdgeMode`, gets the quarter pieces with `edge_quarters`, draws `edge_sprites` (a frame per piece, placed with
//! `EdgeSprite::place`) and adds `edge_blocked_rects`, grown by its actor half width, to its collision world. Plain
//! Rust with no Bevy types except `IVec2`, `Rect` and `Vec2`.
//!
//! A piece is named by its opaque side: `North` is opaque in the top half, `NorthEast` in the top right triangle.

use std::f32::consts::TAU;

use bevy::math::{IVec2, Rect, Vec2};

use crate::map::{Map, PLAYER_START, start_feet};
use crate::outline::EPSILON;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EdgePiece {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
    Center,
}

impl EdgePiece {
    pub const ALL: [EdgePiece; 9] = [
        EdgePiece::North,
        EdgePiece::NorthEast,
        EdgePiece::East,
        EdgePiece::SouthEast,
        EdgePiece::South,
        EdgePiece::SouthWest,
        EdgePiece::West,
        EdgePiece::NorthWest,
        EdgePiece::Center,
    ];

    /// The name of the piece in snake case, for example `north_east`.
    pub fn name(self) -> &'static str {
        match self {
            EdgePiece::North => "north",
            EdgePiece::NorthEast => "north_east",
            EdgePiece::East => "east",
            EdgePiece::SouthEast => "south_east",
            EdgePiece::South => "south",
            EdgePiece::SouthWest => "south_west",
            EdgePiece::West => "west",
            EdgePiece::NorthWest => "north_west",
            EdgePiece::Center => "center",
        }
    }

    /// The direction from the centre of the cell to its opaque part. Zero for `Center`.
    fn opaque_side(self) -> IVec2 {
        match self {
            EdgePiece::North => IVec2::new(0, 1),
            EdgePiece::NorthEast => IVec2::new(1, 1),
            EdgePiece::East => IVec2::new(1, 0),
            EdgePiece::SouthEast => IVec2::new(1, -1),
            EdgePiece::South => IVec2::new(0, -1),
            EdgePiece::SouthWest => IVec2::new(-1, -1),
            EdgePiece::West => IVec2::new(-1, 0),
            EdgePiece::NorthWest => IVec2::new(-1, 1),
            EdgePiece::Center => IVec2::ZERO,
        }
    }
}

/// One edge cell of a map and its piece. `cell` is numbered from the bottom left, like `MapObject.cell`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapEdge {
    pub cell: IVec2,
    pub piece: EdgePiece,
}

/// Step 1 of the brief: the piece from the free sides of a cell. An error names the free sides of a cell that cannot
/// be an edge: two opposite sides (a wall one cell thick) or all four (a single edge cell).
fn piece_for_free_sides(n: bool, e: bool, s: bool, w: bool) -> Result<EdgePiece, &'static str> {
    use EdgePiece::*;
    Ok(match (n, e, s, w) {
        (false, false, false, false) => Center,
        (false, false, true, false) => North,
        (true, false, false, false) => South,
        (false, false, false, true) => East,
        (false, true, false, false) => West,
        (false, false, true, true) => NorthEast,
        (false, true, true, false) => NorthWest,
        (true, false, false, true) => SouthEast,
        (true, true, false, false) => SouthWest,
        // Three free sides: the piece of the closed side.
        (true, true, true, false) => West,
        (false, true, true, true) => North,
        (true, false, true, true) => East,
        (true, true, false, true) => South,
        (true, false, true, false) => return Err("the north and south sides"),
        (false, true, false, true) => return Err("the east and west sides"),
        (true, true, true, true) => return Err("all four sides"),
    })
}

/// Gives each edge cell of a map of `size` cells its piece, with steps 1 and 2 of the brief. A cell outside the map
/// counts as edge. The result is in the order of `cells`. An error gives the first cell that cannot be an edge and its
/// free sides.
pub fn choose_pieces(size: IVec2, cells: &[IVec2]) -> Result<Vec<MapEdge>, (IVec2, &'static str)> {
    let inside = |c: IVec2| c.x >= 0 && c.y >= 0 && c.x < size.x && c.y < size.y;
    let index = |c: IVec2| (c.y * size.x + c.x) as usize;
    let mut is_edge = vec![false; (size.x * size.y) as usize];
    for &cell in cells {
        is_edge[index(cell)] = true;
    }
    let edge_at = |c: IVec2| !inside(c) || is_edge[index(c)];

    let mut first = vec![None; is_edge.len()];
    for &cell in cells {
        let free = |dx: i32, dy: i32| !edge_at(cell + IVec2::new(dx, dy));
        let piece = piece_for_free_sides(free(0, 1), free(1, 0), free(0, -1), free(-1, 0))
            .map_err(|sides| (cell, sides))?;
        first[index(cell)] = Some(piece);
    }

    // Step 2 reads only the pieces of step 1, so a change does not spread.
    let step_one = |c: IVec2| if inside(c) { first[index(c)] } else { None };
    Ok(cells
        .iter()
        .map(|&cell| {
            let piece = first[index(cell)].expect("every edge cell has a piece");
            let along = match piece {
                EdgePiece::North | EdgePiece::South => [IVec2::new(-1, 0), IVec2::new(1, 0)],
                EdgePiece::East | EdgePiece::West => [IVec2::new(0, -1), IVec2::new(0, 1)],
                _ => return MapEdge { cell, piece },
            };
            let joins_other = along.iter().any(|&d| {
                step_one(cell + d).is_some_and(|other| other != EdgePiece::Center && other != piece)
            });
            MapEdge {
                cell,
                piece: if joins_other {
                    EdgePiece::Center
                } else {
                    piece
                },
            }
        })
        .collect())
}

/// The number of stair steps of a corner in a cell `width` wide, for steps about `step` wide. At least 1.
fn step_count(width: f32, step: f32) -> usize {
    ((width / step).round() as usize).max(1)
}

/// The areas that the opaque half of `piece` blocks in `cell`, before the game grows them by its actor half width.
/// A straight piece blocks the half on its side and `Center` the whole cell. A corner blocks a stair of columns about
/// `step` wide under its diagonal: the top of each column meets the diagonal at the middle of the column, so the
/// stair covers exactly half the cell.
pub fn blocked_rects(piece: EdgePiece, cell: Rect, step: f32) -> Vec<Rect> {
    let mid = cell.center();
    let (min, max) = (cell.min, cell.max);
    match piece {
        EdgePiece::Center => vec![cell],
        EdgePiece::North => vec![Rect::new(min.x, mid.y, max.x, max.y)],
        EdgePiece::South => vec![Rect::new(min.x, min.y, max.x, mid.y)],
        EdgePiece::East => vec![Rect::new(mid.x, min.y, max.x, max.y)],
        EdgePiece::West => vec![Rect::new(min.x, min.y, mid.x, max.y)],
        corner => {
            let side = corner.opaque_side();
            let count = step_count(cell.width(), step);
            let width = cell.width() / count as f32;
            let rise = cell.height() / count as f32;
            (0..count)
                .map(|i| {
                    let from_corner = if side.x > 0 { count - 1 - i } else { i };
                    let height = (count - from_corner) as f32 * rise - rise / 2.0;
                    let x0 = min.x + i as f32 * width;
                    let x1 = if i + 1 == count { max.x } else { x0 + width };
                    if side.y > 0 {
                        Rect::new(x0, max.y - height, x1, max.y)
                    } else {
                        Rect::new(x0, min.y, x1, min.y + height)
                    }
                })
                .collect()
        }
    }
}

/// Grows a blocked area sideways by an actor's half width, as the games do for every blocker.
fn grow(rect: Rect, half_width: f32) -> Rect {
    Rect::new(
        rect.min.x - half_width,
        rect.min.y,
        rect.max.x + half_width,
        rect.max.y,
    )
}

fn strictly_inside(rect: Rect, point: Vec2) -> bool {
    point.x > rect.min.x + EPSILON
        && point.x < rect.max.x - EPSILON
        && point.y > rect.min.y + EPSILON
        && point.y < rect.max.y - EPSILON
}

/// Where an actor `half_width` wide starts on `piece` in `cell`: the point nearest the centre of the cell, away from
/// the opaque side, that is outside the blocked areas grown by `half_width`. `None` for `Center`, and when the point
/// would leave the cell.
pub fn start_on_piece(piece: EdgePiece, cell: Rect, step: f32, half_width: f32) -> Option<Vec2> {
    let away = -piece.opaque_side().as_vec2().normalize_or_zero();
    if away == Vec2::ZERO {
        return None;
    }
    let grown: Vec<Rect> = blocked_rects(piece, cell, step)
        .into_iter()
        .map(|r| grow(r, half_width))
        .collect();
    let centre = cell.center();
    let mut point = centre;
    // Each pass leaves one rectangle along the ray, so there are at most as many passes as rectangles.
    for _ in 0..=grown.len() {
        let Some(rect) = grown.iter().find(|r| strictly_inside(**r, point)) else {
            break;
        };
        let exit = |at: f32, low: f32, high: f32, d: f32| match d {
            d if d > 0.0 => (high - at) / d,
            d if d < 0.0 => (low - at) / d,
            _ => f32::INFINITY,
        };
        let t = exit(centre.x, rect.min.x, rect.max.x, away.x)
            .min(exit(centre.y, rect.min.y, rect.max.y, away.y));
        point = centre + away * t;
    }
    let in_cell = point.cmpge(cell.min - EPSILON).all() && point.cmple(cell.max + EPSILON).all();
    in_cell.then_some(point)
}

/// The free cell nearest to `from`: not an edge and without an object. The nearest by straight-line distance wins.
/// Equal distances are broken clockwise from north (north before east before south before west).
pub fn nearest_free_cell(map: &Map, from: IVec2) -> Option<IVec2> {
    let taken =
        |c: IVec2| map.edges.iter().any(|e| e.cell == c) || map.objects.iter().any(|o| o.cell == c);
    let clockwise_from_north = |d: IVec2| (d.x as f32).atan2(d.y as f32).rem_euclid(TAU);
    (0..map.size.y)
        .flat_map(|y| (0..map.size.x).map(move |x| IVec2::new(x, y)))
        .filter(|&c| !taken(c))
        .min_by(|&a, &b| {
            let (da, db) = (a - from, b - from);
            da.length_squared()
                .cmp(&db.length_squared())
                .then(clockwise_from_north(da).total_cmp(&clockwise_from_north(db)))
        })
}

/// What `edge_start_feet` needs from a game: the stair step of a corner and the half width of its player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgeFit {
    pub step: f32,
    pub half_width: f32,
}

/// Where the player's feet start, for a map whose cell (0,0) is at `origin`. On an edge cell, the start is off the
/// blocked area of its piece (`start_on_piece`), or at the centre of the nearest free cell for a `Center` piece.
/// Elsewhere it is the centre of the `player_start` cell, as `start_feet`. `None` when the map has no marker.
///
/// The same start holds in `EdgeMode::Smooth`. A cut only removes blocking. A quarter piece lies inside a quarter,
/// which starts on a centre line of the cell, and the start on a straight piece is on a centre line, so it is never
/// inside a quarter piece. The start on a corner is in the quarter that faces two free cells, and a fill needs two
/// closed sides.
pub fn edge_start_feet(map: &Map, origin: Vec2, step: f32, half_width: f32) -> Option<Vec2> {
    let centre = start_feet(map, origin)?;
    let start = map.objects.iter().find(|o| o.kind == PLAYER_START)?.cell;
    let Some(edge) = map.edges.iter().find(|e| e.cell == start) else {
        return Some(centre);
    };
    let corner = origin + start.as_vec2() * map.cell_size;
    let cell = Rect::from_corners(corner, corner + Vec2::splat(map.cell_size));
    start_on_piece(edge.piece, cell, step, half_width).or_else(|| {
        let free = nearest_free_cell(map, start)?;
        Some(origin + (free.as_vec2() + Vec2::splat(0.5)) * map.cell_size)
    })
}

/// A quarter of a cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quarter {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl Quarter {
    pub const ALL: [Quarter; 4] = [
        Quarter::TopLeft,
        Quarter::TopRight,
        Quarter::BottomLeft,
        Quarter::BottomRight,
    ];

    /// The direction from the centre of the cell to the quarter, `(-1, 1)` for `TopLeft`.
    fn side(self) -> IVec2 {
        match self {
            Quarter::TopLeft => IVec2::new(-1, 1),
            Quarter::TopRight => IVec2::new(1, 1),
            Quarter::BottomLeft => IVec2::new(-1, -1),
            Quarter::BottomRight => IVec2::new(1, -1),
        }
    }

    fn from_side(side: IVec2) -> Quarter {
        Quarter::ALL
            .into_iter()
            .find(|q| q.side() == side)
            .expect("a quarter for every diagonal")
    }

    /// The area of this quarter in `cell`.
    pub fn rect(self, cell: Rect) -> Rect {
        let side = self.side().as_vec2();
        Rect::from_corners(cell.center(), cell.center() + side * cell.half_size())
    }
}

/// A half-size piece in one quarter of a cell, from the quarter rules of the brief. A `cut` replaces that quarter of the
/// cell's own piece. A fill is drawn over the cell's piece, or over a free cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct QuarterPiece {
    pub cell: IVec2,
    pub quarter: Quarter,
    pub piece: EdgePiece,
    pub cut: bool,
}

/// What a quarter of a cell shows in the basic rendering.
#[derive(Clone, Copy, PartialEq)]
enum Fill {
    Opaque,
    Clear,
    /// Half opaque, along the diagonal of this piece.
    Half(EdgePiece),
}

impl Fill {
    /// The quarter on `side` of a cell with `piece`, or of a free cell.
    fn of(piece: Option<EdgePiece>, side: IVec2) -> Fill {
        let Some(piece) = piece else {
            return Fill::Clear;
        };
        let opaque = piece.opaque_side();
        if opaque == IVec2::ZERO {
            return Fill::Opaque;
        }
        // A straight piece is opaque on the quarters on its side. A corner is opaque in the quarter of its corner,
        // clear in the opposite one, and half in the other two.
        let towards = opaque * side;
        let agree = towards.x + towards.y;
        let straight = opaque.x == 0 || opaque.y == 0;
        match (straight, agree) {
            (true, 1) | (false, 2) => Fill::Opaque,
            (true, _) | (false, -2) => Fill::Clear,
            _ => Fill::Half(piece),
        }
    }

    /// True when the whole side of the quarter that faces `dir` is opaque.
    fn opaque_towards(self, dir: IVec2) -> bool {
        match self {
            Fill::Opaque => true,
            Fill::Clear => false,
            Fill::Half(piece) => {
                let opaque = piece.opaque_side();
                dir == IVec2::new(opaque.x, 0) || dir == IVec2::new(0, opaque.y)
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Side {
    Closed,
    Open,
    Neither,
}

/// The corner piece whose opaque corner points along `side`.
fn diagonal(side: IVec2) -> EdgePiece {
    EdgePiece::ALL
        .into_iter()
        .find(|p| p.opaque_side() == side)
        .expect("a corner for every diagonal")
}

/// The quarter rules of the brief on a whole map: cut, fill, and no fill next to a cut. They read the basic rendering
/// (`edges`, from `choose_pieces`) only, so a change does not spread. Top row of quarters first, then left to right.
pub fn quarter_pieces(size: IVec2, edges: &[MapEdge]) -> Vec<QuarterPiece> {
    let quarters = size * 2;
    let mut fills = vec![Fill::Clear; (quarters.x * quarters.y) as usize];
    let index = |q: IVec2| (q.y * quarters.x + q.x) as usize;
    let at = |cell: IVec2, side: IVec2| cell * 2 + (side + IVec2::ONE) / 2;
    for edge in edges {
        for quarter in Quarter::ALL {
            let side = quarter.side();
            fills[index(at(edge.cell, side))] = Fill::of(Some(edge.piece), side);
        }
    }
    let side_of = |q: IVec2, dir: IVec2| {
        let next = q + dir;
        if next.cmplt(IVec2::ZERO).any() || next.cmpge(quarters).any() {
            return Side::Closed;
        }
        let fill = fills[index(next)];
        if fill.opaque_towards(-dir) {
            Side::Closed
        } else if fill == Fill::Clear {
            Side::Open
        } else {
            Side::Neither
        }
    };
    let corners = [
        (IVec2::Y, IVec2::X),
        (IVec2::X, IVec2::NEG_Y),
        (IVec2::NEG_Y, IVec2::NEG_X),
        (IVec2::NEG_X, IVec2::Y),
    ];
    let mut found: Vec<(IVec2, EdgePiece, bool)> = Vec::new();
    for y in (0..quarters.y).rev() {
        for x in 0..quarters.x {
            let q = IVec2::new(x, y);
            let fill = fills[index(q)];
            for (a, b) in corners {
                let (near_a, near_b) = (side_of(q, a), side_of(q, b));
                let (far_a, far_b) = (side_of(q, -a), side_of(q, -b));
                if fill == Fill::Opaque
                    && near_a == Side::Open
                    && near_b == Side::Open
                    && far_a == Side::Closed
                    && far_b == Side::Closed
                {
                    found.push((q, diagonal(-a - b), true));
                } else if fill == Fill::Clear
                    && near_a == Side::Closed
                    && near_b == Side::Closed
                    && far_a != Side::Closed
                    && far_b != Side::Closed
                {
                    found.push((q, diagonal(a + b), false));
                }
            }
        }
    }
    let cut_at = |q: IVec2| found.iter().any(|(at, _, cut)| *cut && *at == q);
    found
        .iter()
        .filter(|(q, _, cut)| {
            *cut || ![IVec2::X, IVec2::Y, IVec2::NEG_X, IVec2::NEG_Y]
                .into_iter()
                .any(|d| cut_at(*q + d))
        })
        .map(|&(q, piece, cut)| QuarterPiece {
            cell: q.div_euclid(IVec2::splat(2)),
            quarter: Quarter::from_side(q.rem_euclid(IVec2::splat(2)) * 2 - IVec2::ONE),
            piece,
            cut,
        })
        .collect()
}

/// How a game draws its edges. Each game chooses one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeMode {
    /// One piece per edge cell (steps 1 and 2 of the brief). Inside corners stay square.
    Basic,
    /// The basic pieces, then the quarter rules: cuts and fills with half-size pieces.
    Smooth,
}

/// The quarter pieces of a map's edges in `mode`: none for `Basic`.
pub fn edge_quarters(mode: EdgeMode, size: IVec2, edges: &[MapEdge]) -> Vec<QuarterPiece> {
    match mode {
        EdgeMode::Basic => Vec::new(),
        EdgeMode::Smooth => quarter_pieces(size, edges),
    }
}

/// One sprite of the edge: the frame of `piece`, drawn in `cell`. With `quarter` and `half`, the whole frame at half
/// size in that quarter. With `quarter` and not `half`, only that quarter of the frame, at full size. With no
/// `quarter`, the whole frame over the whole cell.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EdgeSprite {
    pub cell: IVec2,
    pub piece: EdgePiece,
    pub quarter: Option<Quarter>,
    pub half: bool,
}

/// Where and how to draw an `EdgeSprite`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EdgePlacement {
    /// The world position of the centre of the sprite.
    pub centre: Vec2,
    /// The scale of the frame.
    pub scale: f32,
    /// The part of the frame to draw, in image pixels with y down (`Sprite::rect`). `None` for the whole frame.
    pub source: Option<Rect>,
}

impl EdgeSprite {
    /// The placement for a map whose cell (0,0) is at `origin`, with frames `art_size` pixels square.
    pub fn place(&self, origin: Vec2, cell_size: f32, art_size: f32) -> EdgePlacement {
        let corner = origin + self.cell.as_vec2() * cell_size;
        let cell = Rect::from_corners(corner, corner + Vec2::splat(cell_size));
        let scale = cell_size / art_size;
        let Some(quarter) = self.quarter else {
            return EdgePlacement {
                centre: cell.center(),
                scale,
                source: None,
            };
        };
        let centre = quarter.rect(cell).center();
        if self.half {
            return EdgePlacement {
                centre,
                scale: scale / 2.0,
                source: None,
            };
        }
        // Image rows run down, so the top quarters are the first half of the rows.
        let side = quarter.side().as_vec2();
        let min = Vec2::new((side.x + 1.0) / 2.0, (1.0 - side.y) / 2.0) * art_size / 2.0;
        EdgePlacement {
            centre,
            scale,
            source: Some(Rect::from_corners(min, min + Vec2::splat(art_size / 2.0))),
        }
    }
}

/// The sprites that draw the edges with the quarter rules. A cell with a cut draws only the quarters of its piece that
/// are kept and not clear, so art that reaches past the half of a frame does not show beside the cut. Cells first,
/// in the order of `edges`, then the quarter pieces.
pub fn edge_sprites(edges: &[MapEdge], quarters: &[QuarterPiece]) -> Vec<EdgeSprite> {
    let mut sprites = Vec::new();
    for edge in edges {
        let cut: Vec<Quarter> = quarters
            .iter()
            .filter(|q| q.cut && q.cell == edge.cell)
            .map(|q| q.quarter)
            .collect();
        if cut.is_empty() {
            sprites.push(EdgeSprite {
                cell: edge.cell,
                piece: edge.piece,
                quarter: None,
                half: false,
            });
            continue;
        }
        sprites.extend(
            Quarter::ALL
                .into_iter()
                .filter(|q| !cut.contains(q) && Fill::of(Some(edge.piece), q.side()) != Fill::Clear)
                .map(|q| EdgeSprite {
                    cell: edge.cell,
                    piece: edge.piece,
                    quarter: Some(q),
                    half: false,
                }),
        );
    }
    sprites.extend(quarters.iter().map(|q| EdgeSprite {
        cell: q.cell,
        piece: q.piece,
        quarter: Some(q.quarter),
        half: true,
    }));
    sprites
}

/// The blocked areas of all edges of `map`, whose cell (0,0) is at `origin`, before the game grows them by its actor
/// half width. `quarters` come from `edge_quarters`. `step` is the stair step of a corner, the same for a half-size
/// corner, so that one has half as many steps. Bottom row first.
pub fn edge_blocked_rects(
    map: &Map,
    origin: Vec2,
    quarters: &[QuarterPiece],
    step: f32,
) -> Vec<Rect> {
    let mut cells: Vec<IVec2> = map.edges.iter().map(|e| e.cell).collect();
    cells.extend(quarters.iter().map(|q| q.cell));
    cells.sort_by_key(|c| (c.y, c.x));
    cells.dedup();
    cells
        .into_iter()
        .flat_map(|cell| {
            let piece = map.edges.iter().find(|e| e.cell == cell).map(|e| e.piece);
            let here: Vec<QuarterPiece> = quarters
                .iter()
                .filter(|q| q.cell == cell)
                .copied()
                .collect();
            let corner = origin + cell.as_vec2() * map.cell_size;
            let rect = Rect::from_corners(corner, corner + Vec2::splat(map.cell_size));
            cell_blocked_rects(piece, rect, &here, step)
        })
        .collect()
}

/// The blocked areas of one cell: its piece (`None` for a free cell) without the quarters that are cut, and the
/// half-size stair of each quarter piece of the cell.
fn cell_blocked_rects(
    piece: Option<EdgePiece>,
    cell: Rect,
    quarters: &[QuarterPiece],
    step: f32,
) -> Vec<Rect> {
    let base = piece.map_or_else(Vec::new, |p| blocked_rects(p, cell, step));
    let cut: Vec<Quarter> = quarters
        .iter()
        .filter(|q| q.cut)
        .map(|q| q.quarter)
        .collect();
    let mut rects: Vec<Rect> = if cut.is_empty() {
        base
    } else {
        Quarter::ALL
            .into_iter()
            .filter(|q| !cut.contains(q))
            .flat_map(|q| base.iter().map(move |r| r.intersect(q.rect(cell))))
            .filter(|r| r.width() > EPSILON && r.height() > EPSILON)
            .collect()
    };
    rects.extend(
        quarters
            .iter()
            .flat_map(|q| blocked_rects(q.piece, q.quarter.rect(cell), step)),
    );
    rects
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collision::resolve;
    use crate::map::parse;
    use crate::outline;
    use EdgePiece::*;

    fn short(piece: EdgePiece) -> &'static str {
        match piece {
            North => "n",
            NorthEast => "ne",
            East => "e",
            SouthEast => "se",
            South => "s",
            SouthWest => "sw",
            West => "w",
            NorthWest => "nw",
            Center => "c",
        }
    }

    /// The size and edge cells of rows drawn top row first, one character per cell: `.` is an edge.
    fn drawn(rows: &[&str]) -> (IVec2, Vec<IVec2>) {
        let size = IVec2::new(rows[0].len() as i32, rows.len() as i32);
        let cells = rows
            .iter()
            .enumerate()
            .flat_map(|(row, text)| {
                let y = size.y - 1 - row as i32;
                text.chars()
                    .enumerate()
                    .filter(|(_, c)| *c == '.')
                    .map(move |(x, _)| IVec2::new(x as i32, y))
            })
            .collect();
        (size, cells)
    }

    /// The pieces as rows of short names, top row first, with `_` for free cells.
    fn rendered(size: IVec2, edges: &[MapEdge]) -> Vec<String> {
        (0..size.y)
            .rev()
            .map(|y| {
                (0..size.x)
                    .map(|x| {
                        edges
                            .iter()
                            .find(|e| e.cell == IVec2::new(x, y))
                            .map_or("_", |e| short(e.piece))
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect()
    }

    #[test]
    fn the_free_sides_give_the_piece() {
        let cases = [
            ((false, false, false, false), Ok(Center)),
            ((false, false, true, false), Ok(North)),
            ((true, false, false, false), Ok(South)),
            ((false, false, false, true), Ok(East)),
            ((false, true, false, false), Ok(West)),
            ((false, false, true, true), Ok(NorthEast)),
            ((false, true, true, false), Ok(NorthWest)),
            ((true, false, false, true), Ok(SouthEast)),
            ((true, true, false, false), Ok(SouthWest)),
            ((true, true, true, false), Ok(West)),
            ((false, true, true, true), Ok(North)),
            ((true, false, true, true), Ok(East)),
            ((true, true, false, true), Ok(South)),
            ((true, false, true, false), Err("the north and south sides")),
            ((false, true, false, true), Err("the east and west sides")),
            ((true, true, true, true), Err("all four sides")),
        ];
        for ((n, e, s, w), expected) in cases {
            assert_eq!(
                piece_for_free_sides(n, e, s, w),
                expected,
                "N {n} E {e} S {s} W {w}"
            );
        }
    }

    #[test]
    fn the_brief_example_gives_its_pieces() {
        // The example of "Map edges" in `dev-docs/extensions/map-files.md`. The `@` at (0,8) is on the ring, so it is an edge.
        let (size, cells) = drawn(&[
            "..........",
            ".  ...  ..",
            ".        .",
            ".        .",
            ".        .",
            "..      ..",
            "...    ...",
            ".     ....",
            "...   ....",
            "..........",
        ]);
        let edges = choose_pieces(size, &cells).unwrap();
        let expected = [
            "c n n c c c n n c c",
            "w _ _ ne c nw _ _ ne c",
            "w _ _ _ _ _ _ _ _ e",
            "w _ _ _ _ _ _ _ _ e",
            "w _ _ _ _ _ _ _ _ e",
            "c sw _ _ _ _ _ _ se c",
            "c c w _ _ _ _ se c c",
            "w _ _ _ _ _ se c c c",
            "c c sw _ _ _ c c c c",
            "c c c s s s c c c c",
        ];
        assert_eq!(rendered(size, &edges), expected);
        let order: Vec<IVec2> = edges.iter().map(|e| e.cell).collect();
        assert_eq!(order, cells);
    }

    #[test]
    fn a_wall_one_cell_thick_or_a_lone_cell_is_an_error() {
        let size = IVec2::splat(3);
        let row = [IVec2::new(0, 1), IVec2::new(1, 1), IVec2::new(2, 1)];
        assert_eq!(
            choose_pieces(size, &row),
            Err((IVec2::new(0, 1), "the north and south sides"))
        );
        let column = [IVec2::new(1, 0), IVec2::new(1, 1), IVec2::new(1, 2)];
        assert_eq!(
            choose_pieces(size, &column),
            Err((IVec2::new(1, 0), "the east and west sides"))
        );
        assert_eq!(
            choose_pieces(size, &[IVec2::new(1, 1)]),
            Err((IVec2::new(1, 1), "all four sides"))
        );
    }

    const CELLS: [Rect; 2] = [
        Rect {
            min: Vec2::new(0.0, 0.0),
            max: Vec2::new(64.0, 64.0),
        },
        Rect {
            min: Vec2::new(-100.0, 40.0),
            max: Vec2::new(-68.0, 72.0),
        },
    ];

    #[test]
    fn a_straight_piece_blocks_the_half_on_its_side_and_center_the_whole_cell() {
        for cell in CELLS {
            let (min, max, mid) = (cell.min, cell.max, cell.center());
            let cases = [
                (North, Rect::new(min.x, mid.y, max.x, max.y)),
                (South, Rect::new(min.x, min.y, max.x, mid.y)),
                (East, Rect::new(mid.x, min.y, max.x, max.y)),
                (West, Rect::new(min.x, min.y, mid.x, max.y)),
                (Center, cell),
            ];
            for (piece, expected) in cases {
                assert_eq!(
                    blocked_rects(piece, cell, 16.0),
                    vec![expected],
                    "{piece:?} {cell:?}"
                );
            }
        }
    }

    #[test]
    fn a_corner_blocks_a_stair_under_its_diagonal() {
        // Two columns 32 wide: the columns meet the diagonal at x 16 and 48.
        assert_eq!(
            blocked_rects(NorthEast, CELLS[0], 32.0),
            vec![
                Rect::new(0.0, 48.0, 32.0, 64.0),
                Rect::new(32.0, 16.0, 64.0, 64.0)
            ]
        );
        let cell = Rect::new(10.0, 20.0, 42.0, 52.0);
        assert_eq!(
            blocked_rects(SouthWest, cell, 16.0),
            vec![
                Rect::new(10.0, 20.0, 26.0, 44.0),
                Rect::new(26.0, 20.0, 42.0, 28.0)
            ]
        );
        // A step wider than the cell gives one column, as high as half the cell.
        assert_eq!(
            blocked_rects(NorthWest, cell, 100.0),
            vec![Rect::new(10.0, 36.0, 42.0, 52.0)]
        );
    }

    #[test]
    fn a_corner_stair_covers_half_the_cell_on_its_corner() {
        for cell in CELLS {
            for step in [cell.width() / 4.0, cell.width() / 3.0] {
                for piece in [NorthEast, NorthWest, SouthEast, SouthWest] {
                    let rects = blocked_rects(piece, cell, step);
                    let area: f32 = rects.iter().map(|r| r.width() * r.height()).sum();
                    let half = cell.width() * cell.height() / 2.0;
                    // Sums of a few products of exact binary fractions: far below a pixel.
                    assert!(
                        (area - half).abs() < 1e-3,
                        "{piece:?} {step}: {area} != {half}"
                    );
                    for (i, a) in rects.iter().enumerate() {
                        assert!(cell.contains(a.min) && cell.contains(a.max), "{piece:?}");
                        for b in &rects[i + 1..] {
                            assert!(
                                a.intersect(*b).is_empty(),
                                "{piece:?}: {a:?} overlaps {b:?}"
                            );
                        }
                    }
                    let side = piece.opaque_side().as_vec2();
                    let corner = cell.center() + side * cell.half_size() * 0.99;
                    let opposite = cell.center() - side * cell.half_size() * 0.99;
                    assert!(rects.iter().any(|r| r.contains(corner)), "{piece:?}");
                    assert!(!rects.iter().any(|r| r.contains(opposite)), "{piece:?}");
                }
            }
        }
    }

    fn grown(piece: EdgePiece, cell: Rect, step: f32, half_width: f32) -> Vec<Rect> {
        blocked_rects(piece, cell, step)
            .into_iter()
            .map(|r| grow(r, half_width))
            .collect()
    }

    fn outline_edges(rects: &[Rect]) -> Vec<outline::Edge> {
        outline::build(rects)
            .into_iter()
            .flat_map(|c| c.edges)
            .collect()
    }

    #[test]
    fn the_start_is_off_the_grown_blocked_area_and_cannot_move_into_it() {
        for cell in CELLS {
            for half_width in [3.0, 7.0] {
                for piece in EdgePiece::ALL.into_iter().filter(|p| *p != Center) {
                    let step = cell.width() / 4.0;
                    let start = start_on_piece(piece, cell, step, half_width)
                        .unwrap_or_else(|| panic!("{piece:?} has a start"));
                    let rects = grown(piece, cell, step, half_width);
                    let clear = |p: Vec2| !rects.iter().any(|r| strictly_inside(*r, p));
                    assert!(clear(start), "{piece:?} {cell:?}: {start}");
                    assert!(cell.contains(start), "{piece:?}: {start}");
                    let into = piece.opaque_side().as_vec2().normalize() * cell.width();
                    let moved = resolve(start, start + into, &outline_edges(&rects)).feet;
                    assert!(
                        clear(moved),
                        "{piece:?} {cell:?}: moved from {start} to {moved}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_start_is_the_nearest_point_to_the_centre_off_the_grown_area() {
        let cell = CELLS[0];
        let start = |piece| start_on_piece(piece, cell, 16.0, 14.0);
        assert_eq!(start(North), Some(Vec2::new(32.0, 32.0)));
        assert_eq!(start(West), Some(Vec2::new(46.0, 32.0)));
        assert_eq!(start(East), Some(Vec2::new(18.0, 32.0)));
        // The column from x 32 to 48 reaches down to y 24. Grown, it holds the centre, and the diagonal leaves it
        // through its bottom.
        let corner = start(NorthEast).unwrap();
        assert!(corner.abs_diff_eq(Vec2::new(24.0, 24.0), 1e-4), "{corner}");
        assert_eq!(start(Center), None);
        // Grown by 40, the west half reaches past the east side of the cell.
        assert_eq!(start_on_piece(West, cell, 16.0, 40.0), None);
    }

    #[test]
    fn diagonal_corners_and_the_cell_behind_them_join_without_a_gap() {
        // South east corners at (0,0) and (1,1) touch at a corner. The cell (1,0) behind them is closed.
        let size = 64.0;
        let at = |x: f32, y: f32| Rect::new(x * size, y * size, (x + 1.0) * size, (y + 1.0) * size);
        let mut rects = grown(SouthEast, at(0.0, 0.0), 16.0, 14.0);
        rects.extend(grown(SouthEast, at(1.0, 1.0), 16.0, 14.0));
        rects.extend(grown(Center, at(1.0, 0.0), 16.0, 14.0));
        assert_eq!(outline::build(&rects).len(), 1);
        // From the free side, walk straight at the joint of the two diagonals.
        let start = Vec2::new(40.0, 88.0);
        let moved = resolve(start, Vec2::new(88.0, 40.0), &outline_edges(&rects)).feet;
        assert!(!rects.iter().any(|r| strictly_inside(*r, moved)), "{moved}");
    }

    const KINDS: &[&str] = &["player_start", "tree"];

    /// A 5 by 4 map with `".": edge`, cells of `cell` pixels, rows drawn top row first.
    fn map(cell: f32, rows: [&str; 4]) -> Map {
        let source = format!(
            "---\nmap: t\ncell: {cell}\norigin: bottom-left\nsize: 5x4\nlegend:\n  \".\": edge\n  \"@\": player_start\n  \"t\": tree\n---\n 3 |{}|\n 2 |{}|\n 1 |{}|\n 0 |{}|\n",
            rows[0], rows[1], rows[2], rows[3]
        );
        parse("t.txt", &source, KINDS).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn a_start_on_a_center_piece_goes_to_the_nearest_free_cell() {
        // In the corner: the nearest free cell is one cell away on the diagonal.
        let corner = map(20.0, [". . . . .", ".       .", ".       .", ". . . . @"]);
        assert_eq!(
            nearest_free_cell(&corner, IVec2::new(4, 0)),
            Some(IVec2::new(3, 1))
        );
        assert_eq!(
            edge_start_feet(&corner, Vec2::new(100.0, -50.0), 5.0, 4.0),
            Some(Vec2::new(170.0, -20.0))
        );
        // Behind a wall: two cells away, straight up.
        let wall = map(32.0, [". . . . .", ".       .", ". . . . .", ". . @ . ."]);
        assert_eq!(
            edge_start_feet(&wall, Vec2::new(-64.0, 16.0), 8.0, 4.0),
            Some(Vec2::new(16.0, 96.0))
        );
        // (3,2) and (2,1) are both at a distance of sqrt 5. Clockwise from north, (2,1) to the west north west
        // comes before (3,2) to the north north west. The tree at (1,1) is not free.
        let tie = map(20.0, [". . . . .", ".       .", ". t   . .", ". . . . @"]);
        assert_eq!(
            nearest_free_cell(&tie, IVec2::new(4, 0)),
            Some(IVec2::new(2, 1))
        );
        let untied = map(20.0, [". . . . .", ".       .", ". t t . .", ". . . . @"]);
        assert_eq!(
            nearest_free_cell(&untied, IVec2::new(4, 0)),
            Some(IVec2::new(3, 2))
        );
    }

    #[test]
    fn a_start_on_another_piece_is_off_its_blocked_half() {
        let west = map(20.0, [". . . . .", "@       .", ".       .", ". . . . ."]);
        // Cell (0,2) from (100,-10) to (120,10). The west half ends at 110, grown by 4 at 114.
        assert_eq!(
            edge_start_feet(&west, Vec2::new(100.0, -50.0), 5.0, 4.0),
            Some(Vec2::new(114.0, 0.0))
        );
        // Off the edge, the start is the centre of the cell, as `start_feet`.
        let inside = map(20.0, [". . . . .", ".   @   .", ".       .", ". . . . ."]);
        assert_eq!(
            edge_start_feet(&inside, Vec2::new(100.0, -50.0), 5.0, 4.0),
            Some(Vec2::new(150.0, 0.0))
        );
    }

    fn brief_example() -> (IVec2, Vec<MapEdge>) {
        let (size, cells) = drawn(&[
            "..........",
            ".  ...  ..",
            ".        .",
            ".        .",
            ".        .",
            "..      ..",
            "...    ...",
            ".     ....",
            "...   ....",
            "..........",
        ]);
        (size, choose_pieces(size, &cells).unwrap())
    }

    #[test]
    fn the_quarter_rules_give_the_smooth_example() {
        // `dev-docs/extensions/map-files.md`, rule 30 (cuts and fills of the example).
        use Quarter::*;
        let (size, edges) = brief_example();
        let q = |x: i32, y: i32, quarter: Quarter, piece: EdgePiece, cut: bool| QuarterPiece {
            cell: IVec2::new(x, y),
            quarter,
            piece,
            cut,
        };
        let expected = vec![
            q(0, 9, BottomRight, NorthWest, true),
            q(2, 9, BottomRight, NorthEast, false),
            q(6, 9, BottomLeft, NorthWest, false),
            q(7, 9, BottomRight, NorthEast, false),
            q(9, 7, TopLeft, NorthEast, false),
            q(0, 5, BottomRight, SouthWest, false),
            q(9, 5, BottomLeft, SouthEast, false),
            q(2, 3, TopLeft, SouthWest, true),
            q(2, 3, BottomLeft, NorthWest, true),
            q(0, 2, TopRight, NorthWest, false),
            q(0, 2, BottomRight, SouthWest, false),
            q(3, 0, TopLeft, SouthWest, false),
            q(5, 0, TopRight, SouthEast, false),
        ];
        assert_eq!(quarter_pieces(size, &edges), expected);
    }

    /// The brief example as a parsed map would hold it, with cells of 64.
    fn example_map() -> Map {
        let (size, edges) = brief_example();
        Map {
            name: "example".into(),
            cell_size: 64.0,
            origin: crate::map::Origin::BottomLeft,
            size,
            draw_size: None,
            jitter: 0.0,
            background: None,
            objects: Vec::new(),
            exits: Vec::new(),
            entries: Vec::new(),
            edges,
        }
    }

    #[test]
    fn smooth_blocked_areas_cut_and_fill_and_stay_one_shape() {
        let map = example_map();
        let origin = Vec2::new(-100.0, 40.0);
        let grown = |mode| {
            let quarters = edge_quarters(mode, map.size, &map.edges);
            edge_blocked_rects(&map, origin, &quarters, 16.0)
                .into_iter()
                .map(|r| grow(r, 14.0))
                .collect::<Vec<Rect>>()
        };
        let (basic, smooth) = (grown(EdgeMode::Basic), grown(EdgeMode::Smooth));
        assert_eq!(outline::build(&basic).len(), 1);
        assert_eq!(outline::build(&smooth).len(), 1);
        let inside = |rects: &[Rect], p: Vec2| rects.iter().any(|r| strictly_inside(*r, p));
        // The cut at (0,9) bottom right frees the lower right corner of that quarter.
        let cut = origin + Vec2::new(60.0, 578.0);
        assert!(inside(&basic, cut) && !inside(&smooth, cut));
        // The fill at (2,9) bottom right blocks the upper part of that quarter, below the north half of the cell.
        let fill = origin + Vec2::new(170.0, 604.0);
        assert!(!inside(&basic, fill) && inside(&smooth, fill));
    }

    #[test]
    fn basic_mode_draws_one_whole_frame_per_cell() {
        let map = example_map();
        assert_eq!(edge_quarters(EdgeMode::Basic, map.size, &map.edges), vec![]);
        let sprites = edge_sprites(
            &map.edges,
            &edge_quarters(EdgeMode::Basic, map.size, &map.edges),
        );
        let whole: Vec<EdgeSprite> = map
            .edges
            .iter()
            .map(|e| EdgeSprite {
                cell: e.cell,
                piece: e.piece,
                quarter: None,
                half: false,
            })
            .collect();
        assert_eq!(sprites, whole);
    }

    #[test]
    fn smooth_mode_draws_the_kept_quarters_beside_a_cut_and_a_fill_over_its_cell() {
        use Quarter::*;
        let map = example_map();
        let quarters = edge_quarters(EdgeMode::Smooth, map.size, &map.edges);
        assert_eq!(quarters, quarter_pieces(map.size, &map.edges));
        let sprites = edge_sprites(&map.edges, &quarters);
        let at = |x: i32, y: i32| -> Vec<(EdgePiece, Option<Quarter>, bool)> {
            sprites
                .iter()
                .filter(|s| s.cell == IVec2::new(x, y))
                .map(|s| (s.piece, s.quarter, s.half))
                .collect()
        };
        // A `center` cut at the bottom right keeps its other three quarters.
        assert_eq!(
            at(0, 9),
            vec![
                (Center, Some(TopLeft), false),
                (Center, Some(TopRight), false),
                (Center, Some(BottomLeft), false),
                (NorthWest, Some(BottomRight), true),
            ]
        );
        // A `west` cut on both left quarters: its right quarters are clear, so nothing of the `west` frame is drawn.
        assert_eq!(
            at(2, 3),
            vec![
                (SouthWest, Some(TopLeft), true),
                (NorthWest, Some(BottomLeft), true)
            ]
        );
        // A fill is drawn over the whole frame of its cell.
        assert_eq!(
            at(2, 9),
            vec![(North, None, false), (NorthEast, Some(BottomRight), true)]
        );
        // A cell without quarter pieces keeps its whole frame.
        assert_eq!(at(4, 8), vec![(Center, None, false)]);
    }

    #[test]
    fn a_sprite_is_placed_on_its_cell_or_quarter() {
        // Cell (1,2) of 48 from origin (10,-20) runs from (58,76) to (106,124). Frames are 64, so the scale is 0.75.
        let sprite = |quarter, half| EdgeSprite {
            cell: IVec2::new(1, 2),
            piece: North,
            quarter,
            half,
        };
        let place = |quarter, half| sprite(quarter, half).place(Vec2::new(10.0, -20.0), 48.0, 64.0);
        assert_eq!(
            place(None, false),
            EdgePlacement {
                centre: Vec2::new(82.0, 100.0),
                scale: 0.75,
                source: None
            }
        );
        assert_eq!(
            place(Some(Quarter::BottomRight), true),
            EdgePlacement {
                centre: Vec2::new(94.0, 88.0),
                scale: 0.375,
                source: None
            }
        );
        let cases = [
            (
                Quarter::TopLeft,
                Vec2::new(70.0, 112.0),
                Rect::new(0.0, 0.0, 32.0, 32.0),
            ),
            (
                Quarter::TopRight,
                Vec2::new(94.0, 112.0),
                Rect::new(32.0, 0.0, 64.0, 32.0),
            ),
            (
                Quarter::BottomLeft,
                Vec2::new(70.0, 88.0),
                Rect::new(0.0, 32.0, 32.0, 64.0),
            ),
            (
                Quarter::BottomRight,
                Vec2::new(94.0, 88.0),
                Rect::new(32.0, 32.0, 64.0, 64.0),
            ),
        ];
        for (quarter, centre, source) in cases {
            assert_eq!(
                place(Some(quarter), false),
                EdgePlacement {
                    centre,
                    scale: 0.75,
                    source: Some(source)
                },
                "{quarter:?}"
            );
        }
    }
}

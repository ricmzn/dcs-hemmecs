use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Debug, Formatter},
    fs,
    io::ErrorKind,
    path::PathBuf,
    sync::{
        mpsc::{self, Receiver, Sender},
        RwLock,
    },
    thread::{self, JoinHandle},
};

use glium::{
    glutin::{
        dpi::PhysicalSize,
        event::Event,
        event::{KeyboardInput, VirtualKeyCode, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        platform::windows::{EventLoopExtWindows, WindowExtWindows},
        window::Window,
        window::WindowBuilder,
        ContextBuilder,
    },
    implement_vertex,
    index::PrimitiveType,
    texture::{RawImage2d, Texture2d},
    uniform,
    uniforms::{Sampler, SamplerWrapFunction},
    Depth, DepthTest, Display, DrawParameters, Frame, IndexBuffer, Program, Rect, Surface,
    VertexBuffer,
};

use anyhow::Result;
use image::GenericImageView;
use mpsc::TryRecvError;
use serde::Deserialize;

use crate::{
    consts::{HEIGHT, WIDTH},
    data::{dcs, FlightData},
    installer::DCSVersion,
};

#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: [f32; 3],
}

#[allow(dead_code)]
impl Vertex {
    fn new(x: f32, y: f32, z: f32) -> Vertex {
        Vertex {
            position: [x, y, z],
        }
    }
    fn x(&self) -> f32 {
        self.position[0]
    }
    fn y(&self) -> f32 {
        self.position[1]
    }
    fn z(&self) -> f32 {
        self.position[2]
    }
}

implement_vertex!(Vertex, position);

const VS: &str = r"
#version 140
in vec3 position;
out vec3 vertex_normal;
varying vec3 vertex_pos;

uniform mat4 view_matrix;

void main() {
    vertex_pos = position;
    gl_Position = view_matrix * vec4(position, 1.0);
}
";

const PS: &str = r"
#version 140
out vec4 color;
varying vec3 vertex_pos;

uniform sampler2D tex;

float tex_scale = 4000.0f;
float max_alt1 = 75.0f;
float max_alt2 = 425.0f;
vec4 sea = vec4(0.0, 0.25, 0.75, 1.0);
vec4 beach = vec4(0.75, 0.5, 0.0, 1.0);
vec4 grass = vec4(0.0, 0.8, 0.0, 1.0);
vec4 mountain = vec4(0.8, 0.0, 0.0, 1.0);

void main() {
    if (vertex_pos.y < 0.25) {
        color = sea;
    } else if (vertex_pos.y < max_alt1) {
        color = mix(beach, grass, vertex_pos.y / max_alt1);
    } else if (vertex_pos.y < max_alt2) {
        color = mix(grass, mountain, (vertex_pos.y - max_alt1) / max_alt2);
    } else {
        color = mountain;
    }
    color = color * texture(tex, vec2(vertex_pos.x / tex_scale, vertex_pos.z / tex_scale));
    color = clamp(color, vec4(0.01, 0.01, 0.01, 1.0), vec4(1.0));
}
";

#[derive(Clone, Deserialize)]
struct Tile {
    x: i32,
    z: i32,
    size: i32,
    offset: f32,
    precision: f32,
    terrain: String,
    data: Option<Vec<f32>>,
}

impl Debug for Tile {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Tile")
            .field("x", &self.x)
            .field("z", &self.z)
            .field("size", &self.size)
            .field("precision", &self.precision)
            .field("terrain", &self.terrain)
            .field(
                "data.len()",
                &self
                    .data
                    .as_ref()
                    .map(|data| data.len() as isize)
                    .unwrap_or(-1),
            )
            .finish()
    }
}

/// Order: clockwise
/// Constructs odd rows from the top-left vertex and even rows from the bottom-right one
fn generate_indices(vertex_count: u32, rows: u32, cols: u32) -> Vec<u32> {
    let mut indices = Vec::new();
    for i in 0..vertex_count {
        let row = i / rows;
        let col = i % rows;
        // v[i] as the top-left vertex
        // Skip bottom-most and rightmost vertices
        if row < rows - 1 && col < cols - 1 {
            indices.push(i);
            indices.push(i + 1);
            indices.push(i + cols);
        }
        // v[i] as the bottom-right vertex
        // Skip topmost and leftmost vertices
        if row > 0 && col > 0 {
            indices.push(i - 1);
            indices.push(i - cols);
            indices.push(i);
        }
    }
    indices
}

#[derive(Debug)]
struct Bounds {
    xmin: f32,
    xmax: f32,
    zmin: f32,
    zmax: f32,
}

impl Default for Bounds {
    fn default() -> Self {
        Bounds {
            xmin: f32::NAN,
            xmax: f32::NAN,
            zmin: f32::NAN,
            zmax: f32::NAN,
        }
    }
}

impl Bounds {
    fn update(&mut self, vertex: &Vertex) {
        self.xmin = self.xmin.min(vertex.x());
        self.xmax = self.xmax.max(vertex.x());
        self.zmin = self.zmin.min(vertex.z());
        self.zmax = self.zmax.max(vertex.z());
    }
    fn expand(&mut self, other: &Self) {
        self.xmin = self.xmin.min(other.xmin);
        self.xmax = self.xmax.max(other.xmax);
        self.zmin = self.zmin.min(other.zmin);
        self.zmax = self.zmax.max(other.zmax);
    }
    fn for_tile(size: i32, x: i32, z: i32) -> Self {
        Bounds {
            xmin: (size * x) as f32,
            xmax: (size * (x + 1)) as f32,
            zmin: (size * z) as f32,
            zmax: (size * (z + 1)) as f32,
        }
    }
}

struct GPUTile {
    vertex_buffer: VertexBuffer<Vertex>,
    index_buffer: IndexBuffer<u32>,
    bounds: Bounds,
}

fn distance_2d(a: (f32, f32), b: (f32, f32)) -> f32 {
    f32::sqrt(f32::powi(f32::abs(a.0 - b.0), 2) + f32::powi(f32::abs(a.1 - b.1), 2))
}

fn distance_to(coords: &glm::Vec3, bounds: &Bounds) -> f32 {
    let (nearest_x, nearest_z) = (
        if coords.x < bounds.xmin {
            bounds.xmin
        } else if coords.x < bounds.xmax {
            return 0.0;
        } else {
            bounds.xmax
        },
        if coords.z < bounds.zmin {
            bounds.zmin
        } else if coords.z < bounds.zmax {
            return 0.0;
        } else {
            bounds.zmax
        },
    );
    distance_2d((coords.x, coords.z), (nearest_x, nearest_z))
}

fn tiles_around(
    coords: &glm::Vec3,
    range: f32,
    tile_size: i32,
) -> impl Iterator<Item = (i32, i32)> {
    let tile_size = tile_size;
    let range_tiles = f32::ceil(range / tile_size as f32) as i32;
    let xcenter = coords.x;
    let zcenter = coords.z;
    let xtile = f32::floor(xcenter / tile_size as f32) as i32;
    let ztile = f32::floor(zcenter / tile_size as f32) as i32;
    let mut x = xtile - range_tiles;
    let mut z = ztile - range_tiles;
    std::iter::from_fn(move || {
        if x < xtile + range_tiles {
            if z < ztile + range_tiles {
                let tile;
                if distance_to(
                    &glm::Vec3::new(xcenter, 0.0, zcenter),
                    &Bounds::for_tile(tile_size, x, z),
                ) <= range
                {
                    tile = Some(Some((x as i32, z as i32)));
                } else {
                    tile = Some(None);
                }
                z += 1;
                tile
            } else {
                z = ztile - range_tiles;
                x += 1;
                Some(None)
            }
        } else {
            None
        }
    })
    .flatten()
}

#[allow(dead_code)]
struct TileRequest {
    x: i32,
    z: i32,
    size: i32,
    terrain: String,
}

struct TileMap {
    index_cache: HashMap<u32, Vec<u32>>,
    active_tiles: HashMap<(i32, i32), GPUTile>,
    queued_tiles: HashSet<(i32, i32)>,
    total_bounds: Bounds,

    _w: JoinHandle<()>,
    tx: Sender<TileRequest>,
    rx: Receiver<Tile>,
}

impl Default for TileMap {
    fn default() -> Self {
        let (tx, thread_rx) = mpsc::channel();
        let (thread_tx, rx) = mpsc::channel();
        TileMap {
            index_cache: Default::default(),
            active_tiles: Default::default(),
            queued_tiles: Default::default(),
            total_bounds: Default::default(),
            _w: thread::Builder::new()
                .name(String::from("TileMap worker"))
                .spawn(move || TileMap::worker_func_unwrapper(thread_rx, thread_tx))
                .unwrap(),
            tx,
            rx,
        }
    }
}

impl TileMap {
    const EMPTY_TILE: &'static [f32] = &[0.0, 0.0, 0.0, 0.0];
    const STREAM_RANGE: f32 = 64_000.0;
    const TILE_SIZE: i32 = 16_000;

    fn worker_func_unwrapper(rx: Receiver<TileRequest>, tx: Sender<Tile>) {
        TileMap::worker_func(rx, tx).unwrap();
    }

    fn worker_func(rx: Receiver<TileRequest>, tx: Sender<Tile>) -> Result<()> {
        loop {
            let request = rx.recv()?;
            let path = PathBuf::from(DCSVersion::Stable.user_folder()?.join("tiles").join(
                format!("caucasus_{}_{}_{}.pack", request.size, request.x, request.z),
            ));
            // Load data from disk
            let tile: Tile = match fs::read(path) {
                Ok(bytes) => rmp_serde::from_read_ref(&bytes)?,
                Err(e) => match e.kind() {
                    ErrorKind::NotFound => {
                        println!(
                            "no data available for ({}, {})",
                            request.x, request.z
                        );
                        continue;
                    }
                    _ => Err(e)?,
                },
            };
            // Fill tile with a flat surface if it contains no data
            let tile = match &tile.data {
                Some(_) => tile,
                None => Tile {
                    data: Some(TileMap::EMPTY_TILE.into()),
                    precision: tile.size as f32,
                    ..tile
                },
            };
            println!("loaded tile ({}, {}) from disk", tile.x, tile.z);
            tx.send(tile)?;
        }
    }

    fn create_gpu_tile(
        &mut self,
        tile: &Tile,
        display: &Display,
        x: i32,
        z: i32,
    ) -> Result<GPUTile> {
        print!("processing tile ({}, {})... ", x, z);
        let heights = tile.data.as_ref().unwrap();
        let rows = (tile.size as f32 / tile.precision) as i32 + 1;
        let cols = rows;
        let positions: Vec<_> = heights
            .iter()
            .enumerate()
            .map(|(i, &y)| {
                let i = i as i32;
                Vertex::new(
                    (tile.x * tile.size) as f32 + (i / cols) as f32 * tile.precision,
                    y + tile.offset,
                    (tile.z * tile.size) as f32 + (i % cols) as f32 * tile.precision,
                )
            })
            .collect();

        let indices = self
            .index_cache
            .entry(heights.len() as u32)
            .or_insert_with_key(|&vertex_count| {
                generate_indices(vertex_count, rows as u32, cols as u32)
            });

        let mut bounds = Bounds::default();
        positions.iter().for_each(|pos| bounds.update(pos));

        print!("uploading {} triangles to GPU... ", indices.len() / 3);
        let vbo = VertexBuffer::new(display, &positions)?;
        let ibo = IndexBuffer::new(display, PrimitiveType::TrianglesList, &indices)?;
        println!("done");

        Ok(GPUTile {
            vertex_buffer: vbo,
            index_buffer: ibo,
            bounds: bounds,
        })
    }

    fn update(&mut self, display: &Display, coords: &glm::Vec3) -> Result<()> {
        let mut updated = false;

        // Queue up tiles in range for loading
        for (x, z) in tiles_around(coords, Self::STREAM_RANGE, Self::TILE_SIZE) {
            if !self.active_tiles.contains_key(&(x, z)) && !self.queued_tiles.contains(&(x, z)) {
                self.queued_tiles.insert((x, z));
                self.tx.send(TileRequest {
                    x,
                    z,
                    size: Self::TILE_SIZE,
                    terrain: String::from("caucasus"),
                })?;
            }
        }

        // Add processed tiles from the queue
        loop {
            match self.rx.try_recv() {
                Ok(tile) => {
                    let new_tile = self.create_gpu_tile(&tile, &display, tile.x, tile.z)?;
                    self.active_tiles.insert((tile.x, tile.z), new_tile);
                    self.queued_tiles.remove(&(tile.x, tile.z));
                    updated = true;
                }
                Err(TryRecvError::Empty) => break,
                Err(e) => Err(e)?,
            }
        }

        // Remove tiles out of range
        self.active_tiles.retain(|&(x, z), tile| {
            if distance_to(coords, &tile.bounds) > Self::STREAM_RANGE {
                println!("dropping tile ({}, {})", x, z);
                updated = true;
                false
            } else {
                true
            }
        });

        // Update the bounds
        if updated {
            self.total_bounds = Bounds::default();
            for (_, tile) in &self.active_tiles {
                self.total_bounds.expand(&tile.bounds);
            }
            println!(
                "total loaded tiles: {}, {:?}",
                self.active_tiles.len(),
                self.total_bounds
            );
        }

        Ok(())
    }
}

/// Makes the window transparent and returns the required viewport and scissor rects for drawing in the window
unsafe fn make_transparent(display: &Display) -> (Rect, Rect) {
    let gl_window = display.gl_window();
    let window = &mut *(gl_window.window() as *const _ as *mut Window);

    use winapi::shared::windef::HWND;
    use winapi::um::winuser::*;

    let hwnd = window.hwnd() as HWND;
    let screen_width = GetSystemMetrics(SM_CXSCREEN);
    let screen_height = GetSystemMetrics(SM_CYSCREEN);
    // increase the FOV a bit
    let width = (WIDTH as f32 * 1.2) as i32;
    let height = (HEIGHT as f32 * 1.2) as i32;
    let x = screen_width / 2 - width / 2;
    let y = screen_height / 2 - height / 2 - screen_height / 10;
    SetWindowLongPtrA(
        hwnd,
        GWL_EXSTYLE,
        (WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT) as isize,
    );
    SetLayeredWindowAttributes(hwnd, 0, 64, LWA_ALPHA | LWA_COLORKEY);
    SetWindowPos(hwnd, HWND_TOPMOST, x, y, width, height, 0);
    (
        // left & bottom = frustrum offset
        Rect {
            left: -x as u32,
            bottom: -(screen_height / 2 + screen_height / 10 - height / 2) as u32,
            width: screen_width as u32,
            height: screen_height as u32,
        },
        Rect {
            left: 0,
            bottom: 0,
            width: width as u32,
            height: height as u32,
        },
    )
}

fn draw(
    mut frame: Frame,
    tile_map: &TileMap,
    program: &Program,
    view_matrix: &glm::Mat4,
    draw_params: &DrawParameters,
    texture: &Texture2d,
) -> Result<()> {
    let uniforms = uniform! {
        view_matrix: [
            [ view_matrix[(0, 0)], view_matrix[(1, 0)], view_matrix[(2, 0)], view_matrix[(3, 0)] ],
            [ view_matrix[(0, 1)], view_matrix[(1, 1)], view_matrix[(2, 1)], view_matrix[(3, 1)] ],
            [ view_matrix[(0, 2)], view_matrix[(1, 2)], view_matrix[(2, 2)], view_matrix[(3, 2)] ],
            [ view_matrix[(0, 3)], view_matrix[(1, 3)], view_matrix[(2, 3)], view_matrix[(3, 3)] ],
        ],
        texture: Sampler::new(texture)
            .wrap_function(SamplerWrapFunction::Repeat)
            .anisotropy(8),
    };
    frame.clear_color_and_depth((0.0, 0.0, 0.0, 0.0), 1.0);
    for (_, tile) in &tile_map.active_tiles {
        frame.draw(
            &tile.vertex_buffer,
            &tile.index_buffer,
            &program,
            &uniforms,
            draw_params,
        )?
    }
    Ok(frame.finish()?)
}

pub fn create(data_handle: &RwLock<Option<FlightData>>) {
    let event_loop: EventLoop<()> = EventLoop::new_any_thread();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT))
        .with_decorations(false)
        .with_title("Synthetic Terrain");
    let context = ContextBuilder::new().with_depth_buffer(24).with_vsync(true);
    let mut display = Display::new(window, context, &event_loop).unwrap();
    let (viewport, scissor) = unsafe { make_transparent(&mut display) };
    let program = Program::from_source(&display, VS, PS, None).unwrap();

    let draw_params = DrawParameters {
        viewport: Some(viewport),
        scissor: Some(scissor),
        depth: Depth {
            test: DepthTest::IfLess,
            write: true,
            ..Default::default()
        },
        ..Default::default()
    };

    let texture = {
        let image = image::io::Reader::open("texture.png")
            .unwrap()
            .decode()
            .unwrap();
        let dimensions = image.dimensions();
        let image = RawImage2d::from_raw_rgba_reversed(&image.into_rgba8(), dimensions);
        Texture2d::new(&display, image).unwrap()
    };

    let mut tile_map = TileMap::default();

    // Hack the data reference lifetime away (unsound!)
    let data_handle: &'static RwLock<Option<FlightData>> = unsafe { &*(data_handle as *const _) };

    event_loop.run(move |ev, _, control_flow| match ev {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    },
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => (),
        },
        Event::MainEventsCleared => {
            let fd = { data_handle.read().unwrap().clone() };
            let fd = fd.unwrap_or_else(|| FlightData {
                cam: dcs::Position {
                    x: dcs::Vec3 {
                        x: -0.60237205,
                        y: -0.25983366,
                        z: 0.7547415,
                    },
                    y: dcs::Vec3 {
                        x: 0.07104907,
                        y: 0.9243294,
                        z: 0.37492293,
                    },
                    z: dcs::Vec3 {
                        x: -0.79504734,
                        y: 0.27946675,
                        z: -0.5383293,
                    },
                    p: dcs::Vec3 {
                        x: -48245.492,
                        y: 2335.9749,
                        z: 293213.6,
                    },
                },
                ..FlightData::default()
            });
            let cam_pos = fd.cam.p.as_glm_vec3();
            let cam_fwd = fd.cam.x.as_glm_vec3();
            let cam_up = fd.cam.y.as_glm_vec3();
            let view_matrix = glm::perspective(16.0 / 9.0, f32::to_radians(50.0), 0.5, 50_000.0)
                * glm::look_at_rh(&cam_pos, &(cam_pos + cam_fwd * 100.0), &cam_up);

            tile_map.update(&display, &cam_pos).unwrap();

            draw(
                display.draw(),
                &tile_map,
                &program,
                &view_matrix,
                &draw_params,
                &texture,
            )
            .unwrap();
        }
        _ => (),
    });
}

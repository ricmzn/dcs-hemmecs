use std::{collections::HashMap, fs, sync::RwLock};

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
    uniform, Display, DrawParameters, Frame, IndexBuffer, Program, Rect, Surface, VertexBuffer,
};

use anyhow::Result;
use once_cell::unsync::Lazy;
use serde::Deserialize;

use crate::{
    consts::{HEIGHT, WIDTH},
    data::{dcs, FlightData},
};

#[derive(Copy, Clone, Debug)]
struct Vertex {
    position: [f32; 3],
}

impl Vertex {
    fn new(x: f32, y: f32, z: f32) -> Vertex {
        Vertex {
            position: [x, y, z],
        }
    }
    fn _x(&self) -> f32 {
        self.position[0]
    }
    fn _y(&self) -> f32 {
        self.position[1]
    }
    fn _z(&self) -> f32 {
        self.position[2]
    }
}

implement_vertex!(Vertex, position);

const VS: &str = r"
#version 140
in vec3 position;
uniform mat4 view_matrix;
varying vec3 vertex_pos;

void main() {
    vertex_pos = position;
    gl_Position = view_matrix * vec4(position, 1.0);
}
";

const PS: &str = r"
#version 140
out vec4 color;
varying vec3 vertex_pos;

float max_alt1 = 75.0f;
float max_alt2 = 250.0f;
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
        color = mix(grass, beach, (vertex_pos.y - max_alt1) / max_alt2);
    } else {
        color = mountain;
    }
}
";

#[derive(Clone, Deserialize)]
struct Tile {
    x: f32,
    z: f32,
    size: f32,
    precision: f32,
    terrain: String,
    data: Option<Vec<f32>>,
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

type Tiles = Vec<(VertexBuffer<Vertex>, IndexBuffer<u32>)>;

#[derive(Default, Debug)]
struct Bounds {
    xmin: f32,
    xmax: f32,
    zmin: f32,
    zmax: f32,
}

impl Bounds {
    fn new() -> Bounds {
        Bounds {
            xmin: f32::NAN,
            xmax: f32::NAN,
            zmin: f32::NAN,
            zmax: f32::NAN,
        }
    }
    fn update(&mut self, vertex: &Vertex) {
        self.xmin = self.xmin.min(vertex._x());
        self.xmax = self.xmax.max(vertex._x());
        self.zmin = self.zmin.min(vertex._z());
        self.zmax = self.zmax.max(vertex._z());
    }
}

fn load_tiles(display: &Display) -> Result<(Tiles, Bounds)> {
    let files = fs::read_dir(r"C:\Users\ricmz\Saved Games\DCS\tiles")?;
    let empty = vec![0.0, 0.0, 0.0, 0.0];
    let mut vbos = Vec::new();
    let mut index_cache = HashMap::new();
    let mut bounds = Bounds::new();
    for file in files {
        let bytes = fs::read(file?.path())?;
        let tile: Tile = rmp_serde::from_read_ref(&bytes)?;
        let tile = match &tile.data {
            Some(_) => tile,
            None => Tile {
                data: Some(empty.clone()),
                precision: tile.size,
                ..tile
            },
        };
        let heights = tile.data.as_ref().unwrap();
        print!("Processing tile ({}, {})... ", tile.x, tile.z);
        let rows = (tile.size / tile.precision) as u32 + 1;
        let cols = rows;
        let positions: Vec<_> = heights
            .iter()
            .enumerate()
            .map(|(i, &y)| {
                let i = i as u32;
                Vertex::new(
                    tile.x * tile.size + (i / cols) as f32 * tile.precision,
                    y,
                    tile.z * tile.size + (i % cols) as f32 * tile.precision,
                )
            })
            .collect();
        let indices = index_cache
            .entry(heights.len())
            .or_insert_with_key(|&vertex_count| generate_indices(vertex_count as u32, rows, cols));
        let vbo = VertexBuffer::new(display, &positions)?;
        let ibo = IndexBuffer::new(display, PrimitiveType::TrianglesList, &indices)?;
        positions.iter().for_each(|pos| bounds.update(pos));
        vbos.push((vbo, ibo));
        println!("done");
    }
    println!(
        "Finished creating {} triangles",
        vbos.iter().map(|(_, ibo)| ibo.len() / 3).sum::<usize>()
    );
    Ok((vbos, bounds))
}

const DRAW_PARAMETERS: Lazy<DrawParameters> = Lazy::new(|| DrawParameters {
    viewport: Some(Rect {
        // Negative numbers in hex notation because glium's API only accepts u32...
        bottom: 0xFFFF_FE20, // -480
        left: 0xFFFF_FD00,   // -768
        width: 2560,
        height: 1440,
    }),
    scissor: Some(Rect {
        bottom: 0,
        left: 0,
        width: 1024,
        height: 768,
    }),
    ..Default::default()
});

fn draw(
    mut frame: Frame,
    tiles: &Tiles,
    program: &Program,
    view_matrix: &glm::Mat4,
    _cam: &glm::Vec3,
) {
    let uniforms = uniform! {
        view_matrix: [
            [ view_matrix[(0, 0)], view_matrix[(1, 0)], view_matrix[(2, 0)], view_matrix[(3, 0)] ],
            [ view_matrix[(0, 1)], view_matrix[(1, 1)], view_matrix[(2, 1)], view_matrix[(3, 1)] ],
            [ view_matrix[(0, 2)], view_matrix[(1, 2)], view_matrix[(2, 2)], view_matrix[(3, 2)] ],
            [ view_matrix[(0, 3)], view_matrix[(1, 3)], view_matrix[(2, 3)], view_matrix[(3, 3)] ],
        ],
    };
    frame.clear_color(0.0, 0.0, 0.0, 0.0);
    for tile in tiles {
        frame
            .draw(&tile.0, &tile.1, &program, &uniforms, &DRAW_PARAMETERS)
            .unwrap();
    }
    frame.finish().unwrap();
}

unsafe fn make_transparent(display: &Display) {
    let gl_window = display.gl_window();
    let window = &mut *(gl_window.window() as *const _ as *mut Window);

    use winapi::shared::windef::HWND;
    use winapi::um::winuser::*;

    let hwnd = window.hwnd() as HWND;
    SetWindowLongPtrA(
        hwnd,
        GWL_EXSTYLE,
        (WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TRANSPARENT) as isize,
    );
    SetLayeredWindowAttributes(hwnd, 0, 64, LWA_ALPHA | LWA_COLORKEY);
    SetWindowPos(hwnd, HWND_TOPMOST, 768, 192, WIDTH, HEIGHT, 0);
}

pub fn create(data_handle: &RwLock<Option<FlightData>>) {
    let event_loop: EventLoop<()> = EventLoop::new_any_thread();
    let window = WindowBuilder::new()
        .with_inner_size(PhysicalSize::new(WIDTH, HEIGHT))
        .with_decorations(false)
        .with_title("Synthetic Terrain");
    let context = ContextBuilder::new().with_vsync(true);
    let mut display = Display::new(window, context, &event_loop).unwrap();
    let program = Program::from_source(&display, VS, PS, None).unwrap();
    let (tiles, bounds) = load_tiles(&display).unwrap();

    println!("{:#?}", bounds);
    unsafe { make_transparent(&mut display) }

    // Hack the lifetime away (unsound!)
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
            let view_matrix = glm::perspective(16.0 / 9.0, f32::to_radians(50.0), 0.5, 100000.0)
                * glm::look_at_rh(
                    &fd.cam.p.as_glm_vec3(),
                    &(fd.cam.p.as_glm_vec3() + fd.cam.x.as_glm_vec3() * 100.0),
                    &fd.cam.y.as_glm_vec3(),
                );
            draw(
                display.draw(),
                &tiles,
                &program,
                &view_matrix,
                &fd.cam.p.as_glm_vec3(),
            );
        }
        _ => (),
    });
}

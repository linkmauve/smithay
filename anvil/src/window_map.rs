use std::cell::RefCell;

use smithay::{
    reexports::wayland_server::protocol::wl_surface,
    utils::Rectangle,
    wayland::{
        compositor::{roles::Role, CompositorToken, SubsurfaceRole, TraversalAction},
        shell::{
            legacy::{ShellSurface, ShellSurfaceRole},
            xdg::{ToplevelSurface, XdgSurfaceRole},
        },
    },
};

use crate::shell::SurfaceData;

pub enum Kind<R> {
    Xdg(ToplevelSurface<R>),
    Wl(ShellSurface<R>),
}

impl<R> Kind<R>
where
    R: Role<SubsurfaceRole> + Role<XdgSurfaceRole> + Role<ShellSurfaceRole> + 'static,
{
    pub fn alive(&self) -> bool {
        match *self {
            Kind::Xdg(ref t) => t.alive(),
            Kind::Wl(ref t) => t.alive(),
        }
    }
    pub fn get_surface(&self) -> Option<&wl_surface::WlSurface> {
        match *self {
            Kind::Xdg(ref t) => t.get_surface(),
            Kind::Wl(ref t) => t.get_surface(),
        }
    }

    /// Do this handle and the other one actually refer to the same toplevel surface?
    pub fn equals(&self, other: &Self) -> bool {
        match (self, other) {
            (Kind::Xdg(a), Kind::Xdg(b)) => a.equals(b),
            (Kind::Wl(a), Kind::Wl(b)) => a.equals(b),
            _ => false,
        }
    }
}

struct Window<R> {
    location: (i32, i32),
    /// A bounding box over this window and its children.
    ///
    /// Used for the fast path of the check in `matching`, and as the fall-back for the window
    /// geometry if that's not set explicitly.
    bbox: Rectangle,
    toplevel: Kind<R>,
}

impl<R> Window<R>
where
    R: Role<SubsurfaceRole> + Role<XdgSurfaceRole> + Role<ShellSurfaceRole> + 'static,
{
    /// Finds the topmost surface under this point if any and returns it together with the location of this
    /// surface.
    fn matching(
        &self,
        point: (f64, f64),
        ctoken: CompositorToken<R>,
    ) -> Option<(wl_surface::WlSurface, (f64, f64))> {
        if !self.bbox.contains((point.0 as i32, point.1 as i32)) {
            return None;
        }
        // need to check more carefully
        let found = RefCell::new(None);
        if let Some(wl_surface) = self.toplevel.get_surface() {
            ctoken.with_surface_tree_downward(
                wl_surface,
                self.location,
                |wl_surface, attributes, role, &(mut x, mut y)| {
                    let data = attributes.user_data.get::<SurfaceData>();

                    if let Ok(subdata) = Role::<SubsurfaceRole>::data(role) {
                        x += subdata.location.0;
                        y += subdata.location.1;
                    }

                    let surface_local_point = (point.0 - x as f64, point.1 - y as f64);
                    if data
                        .map(|data| data.contains_point(surface_local_point))
                        .unwrap_or(false)
                    {
                        *found.borrow_mut() = Some((wl_surface.clone(), (x as f64, y as f64)));
                    }

                    TraversalAction::DoChildren((x, y))
                },
                |_, _, _, _| {},
                |_, _, _, _| {
                    // only continue if the point is not found
                    found.borrow().is_none()
                },
            );
        }
        found.into_inner()
    }

    fn self_update(&mut self, ctoken: CompositorToken<R>) {
        let (base_x, base_y) = self.location;
        let (mut min_x, mut min_y, mut max_x, mut max_y) = (base_x, base_y, base_x, base_y);
        if let Some(wl_surface) = self.toplevel.get_surface() {
            ctoken.with_surface_tree_downward(
                wl_surface,
                (base_x, base_y),
                |_, attributes, role, &(mut x, mut y)| {
                    let data = attributes.user_data.get::<SurfaceData>();

                    if let Some((w, h)) = data.and_then(SurfaceData::size) {
                        if let Ok(subdata) = Role::<SubsurfaceRole>::data(role) {
                            x += subdata.location.0;
                            y += subdata.location.1;
                        }

                        // Update the bounding box.
                        min_x = min_x.min(x);
                        min_y = min_y.min(y);
                        max_x = max_x.max(x + w);
                        max_y = max_y.max(y + h);

                        TraversalAction::DoChildren((x, y))
                    } else {
                        // If the parent surface is unmapped, then the child surfaces are hidden as
                        // well, no need to consider them here.
                        TraversalAction::SkipChildren
                    }
                },
                |_, _, _, _| {},
                |_, _, _, _| true,
            );
        }
        self.bbox = Rectangle {
            x: min_x,
            y: min_y,
            width: max_x - min_x,
            height: max_y - min_y,
        };
    }
}

pub struct WindowMap<R> {
    ctoken: CompositorToken<R>,
    windows: Vec<Window<R>>,
}

impl<R> WindowMap<R>
where
    R: Role<SubsurfaceRole> + Role<XdgSurfaceRole> + Role<ShellSurfaceRole> + 'static,
{
    pub fn new(ctoken: CompositorToken<R>) -> Self {
        WindowMap {
            ctoken,
            windows: Vec::new(),
        }
    }

    pub fn insert(&mut self, toplevel: Kind<R>, location: (i32, i32)) {
        let mut window = Window {
            location,
            bbox: Rectangle::default(),
            toplevel,
        };
        window.self_update(self.ctoken);
        self.windows.insert(0, window);
    }

    pub fn get_surface_under(&self, point: (f64, f64)) -> Option<(wl_surface::WlSurface, (f64, f64))> {
        for w in &self.windows {
            if let Some(surface) = w.matching(point, self.ctoken) {
                return Some(surface);
            }
        }
        None
    }

    pub fn get_surface_and_bring_to_top(
        &mut self,
        point: (f64, f64),
    ) -> Option<(wl_surface::WlSurface, (f64, f64))> {
        let mut found = None;
        for (i, w) in self.windows.iter().enumerate() {
            if let Some(surface) = w.matching(point, self.ctoken) {
                found = Some((i, surface));
                break;
            }
        }
        if let Some((i, surface)) = found {
            let winner = self.windows.remove(i);
            self.windows.insert(0, winner);
            Some(surface)
        } else {
            None
        }
    }

    pub fn with_windows_from_bottom_to_top<Func>(&self, mut f: Func)
    where
        Func: FnMut(&Kind<R>, (i32, i32)),
    {
        for w in self.windows.iter().rev() {
            f(&w.toplevel, w.location)
        }
    }

    pub fn refresh(&mut self) {
        self.windows.retain(|w| w.toplevel.alive());
        for w in &mut self.windows {
            w.self_update(self.ctoken);
        }
    }

    pub fn clear(&mut self) {
        self.windows.clear();
    }

    /// Returns the location of the toplevel, if it exists.
    pub fn location(&self, toplevel: &Kind<R>) -> Option<(i32, i32)> {
        self.windows
            .iter()
            .find(|w| w.toplevel.equals(toplevel))
            .map(|w| w.location)
    }

    /// Sets the location of the toplevel, if it exists.
    pub fn set_location(&mut self, toplevel: &Kind<R>, location: (i32, i32)) {
        if let Some(w) = self.windows.iter_mut().find(|w| w.toplevel.equals(toplevel)) {
            w.location = location;
            w.self_update(self.ctoken);
        }
    }
}

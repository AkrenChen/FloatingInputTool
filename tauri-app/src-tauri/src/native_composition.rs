use crate::{native_effects as effects, surface as layout, surface::SurfaceEffects as Parameters};
use windows::{
    core::{Interface, Result, HSTRING},
    System::{DispatcherQueue, DispatcherQueueController},
    Win32::{
        Foundation::{E_POINTER, HWND},
        Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_USE_HOSTBACKDROPBRUSH, DWMWA_WINDOW_CORNER_PREFERENCE,
            DWMWCP_DONOTROUND,
        },
        System::WinRT::{
            Composition::ICompositorDesktopInterop, CreateDispatcherQueueController,
            DispatcherQueueOptions, DQTAT_COM_NONE, DQTYPE_THREAD_CURRENT,
        },
    },
    UI::{
        Color,
        Composition::{
            CompositionColorBrush, CompositionEffectSourceParameter,
            CompositionRoundedRectangleGeometry, Compositor, ContainerVisual,
            Desktop::DesktopWindowTarget, SpriteVisual,
        },
    },
};
use windows_numerics::{Vector2, Vector3};

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Applied {
    active: bool,
    blur: f32,
    tint: f32,
    interface_opacity: f32,
    material_opacity: f32,
    shadow_blur: f32,
    shadow_opacity: f32,
}

pub struct Scene {
    _target: DesktopWindowTarget,
    compositor: Compositor,
    _root: ContainerVisual,
    content: ContainerVisual,
    blur: SpriteVisual,
    tint: SpriteVisual,
    tint_brush: CompositionColorBrush,
    geometry: CompositionRoundedRectangleGeometry,
    shadow_host: crate::native_shadow::ShadowHost,
    shadow_blur: f32,
    shadow_opacity: f32,
    deviation: f32,
    size: Vector2,
    scale: f32,
    active: bool,
    interface_opacity: f32,
    tint_color: [u8; 3],
    _queue: Option<DispatcherQueueController>,
}

impl Scene {
    pub fn new(hwnd: HWND, width: f32, height: f32, scale: f32) -> Result<Self> {
        let _queue = match DispatcherQueue::GetForCurrentThread() {
            Ok(_) => None,
            // A missing queue is returned as S_OK + null; windows-rs projects that as an error.
            Err(error) if error.code() == E_POINTER || error.code().0 == 0 => Some(unsafe {
                CreateDispatcherQueueController(DispatcherQueueOptions {
                    dwSize: std::mem::size_of::<DispatcherQueueOptions>() as u32,
                    threadType: DQTYPE_THREAD_CURRENT,
                    apartmentType: DQTAT_COM_NONE,
                })?
            }),
            Err(error) => return Err(error),
        };
        unsafe {
            let enabled = 1i32;
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_USE_HOSTBACKDROPBRUSH,
                (&enabled as *const i32).cast(),
                4,
            )?;
            DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &DWMWCP_DONOTROUND as *const _ as *const std::ffi::c_void,
                4,
            )?;
        }
        let compositor = Compositor::new()?;
        let interop: ICompositorDesktopInterop = compositor.cast()?;
        let _target = unsafe { interop.CreateDesktopWindowTarget(hwnd, false)? };
        let _root = compositor.CreateContainerVisual()?;
        _target.SetRoot(&_root)?;

        let geometry = compositor.CreateRoundedRectangleGeometry()?;
        let shadow_host = crate::native_shadow::ShadowHost::new(hwnd.0).map_err(|error| {
            windows::core::Error::new(windows::core::HRESULT(0x80004005u32 as i32), error)
        })?;

        let content = compositor.CreateContainerVisual()?;
        let clip = compositor.CreateGeometricClipWithGeometry(&geometry)?;
        content.SetClip(&clip)?;
        let blur = compositor.CreateSpriteVisual()?;
        content.Children()?.InsertAtTop(&blur)?;
        let tint_brush = compositor.CreateColorBrush()?;
        let tint = compositor.CreateSpriteVisual()?;
        tint.SetBrush(&tint_brush)?;
        content.Children()?.InsertAtTop(&tint)?;
        _root.Children()?.InsertAtTop(&content)?;
        let mut scene = Self {
            _queue,
            _target,
            compositor,
            _root,
            content,
            blur,
            tint,
            tint_brush,
            geometry,
            shadow_host,
            shadow_blur: 0.0,
            shadow_opacity: 0.0,
            deviation: -1.0,
            size: Vector2::default(),
            scale,
            active: true,
            interface_opacity: 1.0,
            tint_color: [246, 246, 251],
        };
        scene.resize(width, height, scale)?;
        scene.apply(&Parameters::default(), true)?;
        Ok(scene)
    }

    pub fn resize(&mut self, width: f32, height: f32, scale: f32) -> Result<()> {
        self.scale = scale;
        let (margin, content_width, content_height) = layout::content_bounds(width, height, scale);
        self.size = Vector2 {
            X: content_width,
            Y: content_height,
        };
        self.geometry.SetSize(self.size)?;
        for visual in [&self.blur, &self.tint] {
            visual.SetSize(self.size)?;
        }

        self.content.SetSize(self.size)?;
        let offset = Vector3 {
            X: margin,
            Y: margin,
            Z: 0.0,
        };
        self.content.SetOffset(offset)?;

        Ok(())
    }

    pub fn apply(&mut self, parameters: &Parameters, focused: bool) -> Result<()> {
        let active = match parameters.preview.as_str() {
            "focused" => true,
            "idle" => false,
            _ => focused,
        };
        let deviation = (if active {
            parameters.focused_blur
        } else {
            parameters.idle_blur
        }) * self.scale;
        if deviation != self.deviation {
            let source = CompositionEffectSourceParameter::Create(&HSTRING::from("desktop"))?;
            let effect = effects::gaussian(source.cast()?, deviation);
            let factory = self.compositor.CreateEffectFactory(&effect)?;
            let brush = factory.CreateBrush()?;
            brush.SetSourceParameter(
                &HSTRING::from("desktop"),
                &self.compositor.CreateHostBackdropBrush()?,
            )?;
            self.blur.SetBrush(&brush)?;
            self.deviation = deviation;
        }
        let opacity = if active {
            parameters.focused_tint
        } else {
            parameters.idle_tint
        };
        self.tint_brush.SetColor(Color {
            A: (opacity * 255.0).round() as u8,
            R: self.tint_color[0],
            G: self.tint_color[1],
            B: self.tint_color[2],
        })?;
        self.shadow_host
            .update(parameters, active, self.scale)
            .map_err(|error| {
                windows::core::Error::new(windows::core::HRESULT(0x80004005u32 as i32), error)
            })?;
        self.shadow_blur = parameters.shadow_blur;
        self.shadow_opacity = if active {
            parameters.shadow_opacity
        } else {
            parameters.idle_shadow_opacity
        };
        self.geometry.SetCornerRadius(Vector2 {
            X: parameters.corner_radius * self.scale,
            Y: parameters.corner_radius * self.scale,
        })?;
        self.active = active;
        self.interface_opacity = if active {
            parameters.focused_opacity
        } else {
            parameters.idle_opacity
        };
        self.content.SetOpacity(if active {
            parameters.focused_material_opacity
        } else {
            parameters.idle_material_opacity
        })?;
        Ok(())
    }

    pub fn readback(&self) -> Result<Applied> {
        Ok(Applied {
            active: self.active,
            blur: self.deviation / self.scale,
            tint: self.tint_brush.Color()?.A as f32 / 255.0,
            interface_opacity: self.interface_opacity,
            material_opacity: self.content.Opacity()?,
            shadow_blur: self.shadow_blur,
            shadow_opacity: self.shadow_opacity,
        })
    }

    pub fn set_color(&mut self, color: [u8; 3]) {
        self.tint_color = color;
    }

    pub fn set_docked(&mut self, docked: bool) -> Result<()> {
        self.shadow_host.set_docked(docked);
        self._root.SetIsVisible(!docked)
    }
}

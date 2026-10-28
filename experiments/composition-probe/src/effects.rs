use std::cell::RefCell;
use windows::{
    core::{implement, Error, Interface, Result, GUID, HSTRING, PCWSTR},
    Foundation::{IPropertyValue, PropertyValue},
    Graphics::Effects::{
        IGraphicsEffect, IGraphicsEffectSource, IGraphicsEffectSource_Impl, IGraphicsEffect_Impl,
    },
    Win32::{
        Foundation::E_INVALIDARG,
        Graphics::Direct2D::CLSID_D2D1GaussianBlur,
        System::WinRT::Graphics::Direct2D::{
            IGraphicsEffectD2D1Interop, IGraphicsEffectD2D1Interop_Impl,
            GRAPHICS_EFFECT_PROPERTY_MAPPING, GRAPHICS_EFFECT_PROPERTY_MAPPING_DIRECT,
        },
    },
};

#[implement(IGraphicsEffect, IGraphicsEffectSource, IGraphicsEffectD2D1Interop)]
struct BlurEffect {
    name: RefCell<HSTRING>,
    source: IGraphicsEffectSource,
    deviation: f32,
}

pub fn gaussian(source: IGraphicsEffectSource, deviation: f32) -> IGraphicsEffect {
    BlurEffect {
        name: RefCell::new(HSTRING::from("DesktopBlur")),
        source,
        deviation,
    }
    .into()
}

impl IGraphicsEffectSource_Impl for BlurEffect_Impl {}

impl IGraphicsEffect_Impl for BlurEffect_Impl {
    fn Name(&self) -> Result<HSTRING> {
        Ok(self.name.borrow().clone())
    }
    fn SetName(&self, name: &HSTRING) -> Result<()> {
        *self.name.borrow_mut() = name.clone();
        Ok(())
    }
}

impl IGraphicsEffectD2D1Interop_Impl for BlurEffect_Impl {
    fn GetEffectId(&self) -> Result<GUID> {
        Ok(CLSID_D2D1GaussianBlur)
    }
    fn GetNamedPropertyMapping(
        &self,
        name: &PCWSTR,
        index: *mut u32,
        mapping: *mut GRAPHICS_EFFECT_PROPERTY_MAPPING,
    ) -> Result<()> {
        let property = unsafe { name.to_string()? };
        let property_index = match property.as_str() {
            "StandardDeviation" => 0,
            "Optimization" => 1,
            "BorderMode" => 2,
            _ => return Err(Error::from_hresult(E_INVALIDARG)),
        };
        if index.is_null() || mapping.is_null() {
            return Err(Error::from_hresult(E_INVALIDARG));
        }
        unsafe {
            *index = property_index;
            *mapping = GRAPHICS_EFFECT_PROPERTY_MAPPING_DIRECT;
        }
        Ok(())
    }
    fn GetPropertyCount(&self) -> Result<u32> {
        Ok(3)
    }
    fn GetProperty(&self, index: u32) -> Result<IPropertyValue> {
        match index {
            0 => PropertyValue::CreateSingle(self.deviation)?.cast(),
            1 => PropertyValue::CreateUInt32(1)?.cast(),
            2 => PropertyValue::CreateUInt32(1)?.cast(),
            _ => Err(Error::from_hresult(E_INVALIDARG)),
        }
    }
    fn GetSource(&self, index: u32) -> Result<IGraphicsEffectSource> {
        if index == 0 {
            Ok(self.source.clone())
        } else {
            Err(Error::from_hresult(E_INVALIDARG))
        }
    }
    fn GetSourceCount(&self) -> Result<u32> {
        Ok(1)
    }
}

use windows::core::Interface;
use windows::Graphics::Capture::{
    Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession,
};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::System::WinRT::Direct3D11::{
    CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess,
};

pub struct CaptureSession {
    pub item: GraphicsCaptureItem,
    pub frame_pool: Direct3D11CaptureFramePool,
    pub session: GraphicsCaptureSession,
    pub d3d_device: ID3D11Device,
    pub context: ID3D11DeviceContext,
}

impl CaptureSession {
    pub fn new(item: GraphicsCaptureItem) -> Result<Self, Box<dyn std::error::Error>> {
        unsafe {
            let mut d3d_device: Option<ID3D11Device> = None;
            let mut context: Option<ID3D11DeviceContext> = None;

            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                windows::Win32::Foundation::HMODULE::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut d3d_device),
                None,
                Some(&mut context),
            )?;

            let d3d_device = d3d_device.unwrap();
            let context = context.unwrap();

            let dxgi_device: windows::Win32::Graphics::Dxgi::IDXGIDevice = d3d_device.cast()?;
            let inspectable = CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)?;
            let device: IDirect3DDevice = inspectable.cast()?;

            let size = item.Size()?;
            let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
                &device,
                DirectXPixelFormat::B8G8R8A8UIntNormalized,
                2,
                size,
            )?;

            let session = frame_pool.CreateCaptureSession(&item)?;
            session.StartCapture()?;

            Ok(Self {
                item,
                frame_pool,
                session,
                d3d_device,
                context,
            })
        }
    }

    pub fn capture_frame_to_wgpu(
        &self,
        queue: &wgpu::Queue,
        target_texture: &wgpu::Texture,
    ) -> Result<(), Box<dyn std::error::Error>> {
        unsafe {
            let frame = match self.frame_pool.TryGetNextFrame() {
                Ok(f) => f,
                Err(e) => return Err(format!("TryGetNextFrame failed: {:?}", e).into()),
            };

            let surface = match frame.Surface() {
                Ok(s) => s,
                Err(e) => return Err(format!("Frame Surface failed: {:?}", e).into()),
            };

            // WinRT IDirect3DSurface -> Win32 ID3D11Texture2D の変換には
            // IDirect3DDxgiInterfaceAccess を介す必要がある (E_NOINTERFACE 回避)
            let interop: IDirect3DDxgiInterfaceAccess = surface.cast()?;
            let d3d_texture: ID3D11Texture2D = interop.GetInterface()?;

            let mut desc = D3D11_TEXTURE2D_DESC::default();
            d3d_texture.GetDesc(&mut desc);

            if desc.Width == 0 || desc.Height == 0 {
                return Err("Captured texture has 0 size".into());
            }

            let staging_desc = D3D11_TEXTURE2D_DESC {
                Width: desc.Width,
                Height: desc.Height,
                MipLevels: 1,
                ArraySize: 1,
                Format: desc.Format,
                SampleDesc: desc.SampleDesc,
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0,
            };

            let mut staging_texture: Option<ID3D11Texture2D> = None;
            self.d3d_device
                .CreateTexture2D(&staging_desc, None, Some(&mut staging_texture))?;
            let staging_texture = staging_texture.ok_or("Failed to create staging texture")?;

            // Resource にキャストして Copy/Map に渡す
            let src_resource: ID3D11Resource = d3d_texture.cast()?;
            let dst_resource: ID3D11Resource = staging_texture.cast()?;

            self.context.CopyResource(&dst_resource, &src_resource);

            let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
            self.context
                .Map(&dst_resource, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;

            let row_pitch = mapped.RowPitch;
            let data_size = (desc.Height * row_pitch) as usize;
            let data = std::slice::from_raw_parts(mapped.pData as *const u8, data_size);

            // テクスチャのサイズがキャプチャデータと異なる場合は、キャプチャデータ側に合わせる
            // (本来は Renderer 側でキャプチャサイズに合わせてリサイズしておくのが理想)
            let copy_width = desc.Width.min(target_texture.width());
            let copy_height = desc.Height.min(target_texture.height());

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: target_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                data,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(row_pitch),
                    rows_per_image: Some(desc.Height),
                },
                wgpu::Extent3d {
                    width: copy_width,
                    height: copy_height,
                    depth_or_array_layers: 1,
                },
            );

            self.context.Unmap(&dst_resource, 0);
        }
        Ok(())
    }
}

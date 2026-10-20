#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, strum::EnumString, strum::AsRefStr, strum::Display)]
pub enum ImageResizeType {
    #[default]
    #[strum(serialize = "unknown")]
    Unknown,
    /// The target location and size of the resized image has been manually set.
    #[strum(serialize = "manual")]
    Manual,
    /// Crops the resized image to fit the bounds of its container.
    #[strum(serialize = "crop")]
    Crop,
    /// Pads the resized image to fit the bounds of its container. If only one dimension is passed, will maintain the original aspect ratio.
    #[strum(serialize = "pad")]
    Pad,
    /// Fits the image within the bounds of its container while keeping the aspect ratio. Never enlarges the image and adds no padding.
    #[strum(serialize = "fit")]
    Fit,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, strum::EnumString, strum::AsRefStr, strum::Display)]
pub enum ImageResamplerType {
    #[default]
    #[strum(serialize = "unknown")]
    Unknown,
    /// Lanczos with window 3
    #[strum(serialize = "lanczos3")]
    Lanczos3,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, strum::EnumString, strum::AsRefStr, strum::Display)]
pub enum ImageAnchorPositionType {
    #[default]
    #[strum(serialize = "unknown")]
    Unknown,
    /// Anchors the position of the image to the bottom of it's bounding container.
    #[strum(serialize = "bottom")]
    Bottom,
    /// Anchors the position of the image to the bottom left side of it's bounding container.
    #[strum(serialize = "bottom_left")]
    BottomLeft,
    /// Anchors the position of the image to the bottom right side of it's bounding container.
    #[strum(serialize = "bottom_right")]
    BottomRight,
    /// Anchors the position of the image to the center of it's bounding container.
    #[strum(serialize = "center")]
    Center,
    /// Anchors the position of the image to the left of it's bounding container.
    #[strum(serialize = "left")]
    Left,
    /// Anchors the position of the image to the right of it's bounding container.
    #[strum(serialize = "right")]
    Right,
    /// Anchors the position of the image to the top of it's bounding container.
    #[strum(serialize = "top")]
    Top,
    /// Anchors the position of the image to the top left side of it's bounding container.
    #[strum(serialize = "top_left")]
    TopLeft,
    ///Anchors the position of the image to the top right side of it's bounding container.
    #[strum(serialize = "top_right")]
    TopRight,
}

#[allow(unused)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, strum::EnumString, strum::AsRefStr, strum::Display)]
pub enum ImageFormatType {
    #[default]
    #[strum(serialize = "unknown")]
    Unknown,
    #[strum(serialize = "png")]
    Png,
    #[strum(serialize = "jpeg")]
    Jpeg,
    #[strum(serialize = "webp")]
    Webp,
}

use clap::ValueEnum;
use plotters::prelude::RGBColor;
use std::{fmt, str::FromStr};

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, ValueEnum)]
pub enum AvailableBaselines {
    Runc,
    Kata,
    Snp,
    SnpSc2,
    Tdx,
    TdxSc2,
}

impl fmt::Display for AvailableBaselines {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AvailableBaselines::Runc => write!(f, "runc"),
            AvailableBaselines::Kata => write!(f, "kata"),
            AvailableBaselines::Snp => write!(f, "snp"),
            AvailableBaselines::SnpSc2 => write!(f, "snp-sc2"),
            AvailableBaselines::Tdx => write!(f, "tdx"),
            AvailableBaselines::TdxSc2 => write!(f, "tdx-sc2"),
        }
    }
}

impl FromStr for AvailableBaselines {
    type Err = ();

    fn from_str(input: &str) -> Result<AvailableBaselines, Self::Err> {
        match input {
            "runc" => Ok(AvailableBaselines::Runc),
            "kata" => Ok(AvailableBaselines::Kata),
            "snp" => Ok(AvailableBaselines::Snp),
            "snp-sc2" => Ok(AvailableBaselines::SnpSc2),
            "tdx" => Ok(AvailableBaselines::Tdx),
            "tdx-sc2" => Ok(AvailableBaselines::TdxSc2),
            _ => Err(()),
        }
    }
}

impl AvailableBaselines {
    pub fn iter_variants() -> std::slice::Iter<'static, AvailableBaselines> {
        static VARIANTS: [AvailableBaselines; 6] = [
            AvailableBaselines::Runc,
            AvailableBaselines::Kata,
            AvailableBaselines::Snp,
            AvailableBaselines::SnpSc2,
            AvailableBaselines::Tdx,
            AvailableBaselines::TdxSc2,
        ];
        VARIANTS.iter()
    }

    pub fn get_color(&self) -> RGBColor {
        match self {
            AvailableBaselines::Runc => RGBColor(122, 92, 117),
            AvailableBaselines::Kata => RGBColor(171, 222, 230),
            AvailableBaselines::Snp => RGBColor(203, 170, 203),
            AvailableBaselines::SnpSc2 => RGBColor(213, 160, 163),
            AvailableBaselines::Tdx => RGBColor(255, 255, 181),
            AvailableBaselines::TdxSc2 => RGBColor(205, 255, 101),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, ValueEnum)]
pub enum ImagePullBaselines {
    GuestPull,
    GuestLazy,
    HostMount,
    Sc2,
}

impl fmt::Display for ImagePullBaselines {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImagePullBaselines::GuestPull => write!(f, "guest-pull"),
            ImagePullBaselines::GuestLazy => write!(f, "guest-lazy"),
            ImagePullBaselines::HostMount => write!(f, "host-mount"),
            ImagePullBaselines::Sc2 => write!(f, "sc2"),
        }
    }
}

impl FromStr for ImagePullBaselines {
    type Err = ();

    fn from_str(input: &str) -> Result<ImagePullBaselines, Self::Err> {
        match input {
            "guest-pull" => Ok(ImagePullBaselines::GuestPull),
            "guest-lazy" => Ok(ImagePullBaselines::GuestLazy),
            "host-mount" => Ok(ImagePullBaselines::HostMount),
            "sc2" => Ok(ImagePullBaselines::Sc2),
            _ => Err(()),
        }
    }
}

impl ImagePullBaselines {
    pub fn iter_variants() -> std::slice::Iter<'static, ImagePullBaselines> {
        static VARIANTS: [ImagePullBaselines; 4] = [
            ImagePullBaselines::GuestPull,
            ImagePullBaselines::GuestLazy,
            ImagePullBaselines::HostMount,
            ImagePullBaselines::Sc2,
        ];
        VARIANTS.iter()
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, ValueEnum)]
pub enum StartUpFlavours {
    Cold,
    Warm,
}

impl fmt::Display for StartUpFlavours {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StartUpFlavours::Cold => write!(f, "cold"),
            StartUpFlavours::Warm => write!(f, "warm"),
        }
    }
}

impl FromStr for StartUpFlavours {
    type Err = ();

    fn from_str(input: &str) -> Result<StartUpFlavours, Self::Err> {
        match input {
            "cold" => Ok(StartUpFlavours::Cold),
            "warm" => Ok(StartUpFlavours::Warm),
            _ => Err(()),
        }
    }
}

impl StartUpFlavours {
    pub fn iter_variants() -> std::slice::Iter<'static, StartUpFlavours> {
        static VARIANTS: [StartUpFlavours; 2] = [StartUpFlavours::Cold, StartUpFlavours::Warm];
        VARIANTS.iter()
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, ValueEnum)]
pub enum ImagePullWorkloads {
    Fio,
    HelloWorld,
    TfInference,
}

impl fmt::Display for ImagePullWorkloads {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImagePullWorkloads::Fio => write!(f, "fio"),
            ImagePullWorkloads::HelloWorld => write!(f, "hello-world"),
            ImagePullWorkloads::TfInference => write!(f, "tf-inference"),
        }
    }
}

impl FromStr for ImagePullWorkloads {
    type Err = ();

    fn from_str(input: &str) -> Result<ImagePullWorkloads, Self::Err> {
        match input {
            "fio" => Ok(ImagePullWorkloads::Fio),
            "hello-world" => Ok(ImagePullWorkloads::HelloWorld),
            "tf-inference" => Ok(ImagePullWorkloads::TfInference),
            _ => Err(()),
        }
    }
}

impl ImagePullWorkloads {
    pub fn iter_variants() -> std::slice::Iter<'static, ImagePullWorkloads> {
        static VARIANTS: [ImagePullWorkloads; 3] = [
            ImagePullWorkloads::Fio,
            ImagePullWorkloads::HelloWorld,
            ImagePullWorkloads::TfInference,
        ];
        VARIANTS.iter()
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, ValueEnum)]
pub enum ImagePullEncryptionTypes {
    Encrypted,
    UnEncrypted,
}

impl fmt::Display for ImagePullEncryptionTypes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ImagePullEncryptionTypes::Encrypted => write!(f, "encrypted"),
            ImagePullEncryptionTypes::UnEncrypted => write!(f, "unencrypted"),
        }
    }
}

impl FromStr for ImagePullEncryptionTypes {
    type Err = ();

    fn from_str(input: &str) -> Result<ImagePullEncryptionTypes, Self::Err> {
        match input {
            "encrypted" => Ok(ImagePullEncryptionTypes::Encrypted),
            "unencrypted" => Ok(ImagePullEncryptionTypes::UnEncrypted),
            _ => Err(()),
        }
    }
}

impl ImagePullEncryptionTypes {
    pub fn iter_variants() -> std::slice::Iter<'static, ImagePullEncryptionTypes> {
        static VARIANTS: [ImagePullEncryptionTypes; 2] = [
            ImagePullEncryptionTypes::Encrypted,
            ImagePullEncryptionTypes::UnEncrypted,
        ];
        VARIANTS.iter()
    }
}

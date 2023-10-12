use paddle::{AssetBundle, ImageDesc};

pub(crate) struct Images {
    pub fermyon: ImageDesc,
    pub web: ImageDesc,
    pub home: ImageDesc,
    pub settings: ImageDesc,
    pub stats: ImageDesc,
    pub screen: ImageDesc,
    pub worker: ImageDesc,
    pub loading: ImageDesc,
}

impl Images {
    pub(crate) fn load() -> Self {
        let mut loader = AssetBundle::new();
        let fermyon = ImageDesc::from_path("assets/fermyon.png");
        let web = ImageDesc::from_path("assets/web.svg");
        let home = ImageDesc::from_path("assets/home.svg");
        let settings = ImageDesc::from_path("assets/gear.svg");
        let stats = ImageDesc::from_path("assets/meter.svg");
        let screen = ImageDesc::from_path("assets/device-desktop.svg");
        let worker = ImageDesc::from_path("assets/suitcase.svg");
        let loading = ImageDesc::from_path("assets/loading.svg");

        loader.add_images(&[fermyon, web, home, settings, stats, screen, worker, loading]);
        loader.load();

        Self {
            fermyon,
            web,
            home,
            settings,
            stats,
            screen,
            worker,
            loading,
        }
    }
}

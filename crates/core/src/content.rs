pub const NAME: &str = "AfkaraLP";
pub const TAGLINE: &str = "rust · nix · ml · future bounce";
pub const HEADLINE: &str = "Developer & Creator";
pub const SUBHEADLINE: &str = "Bits, bytes and the occasional digital noise";

pub const ABOUT: &[&str] = &[
    "I'm AfkaraLP. I make music and software, mostly Future Bounce on the music side. \
     I'm currently doing a Bachelor's in Data Science.",
    "Rust is my main language. I use NixOS (btw) and like functional programming. \
     Outside of that: sports and cooking.",
];

pub const CONTACT_TEXT: &str = "Questions, commissions or just saying hi:";

pub struct Contact {
    pub label: &'static str,
    pub value: &'static str,
    pub url: &'static str,
}

pub const CONTACTS: &[Contact] = &[
    Contact {
        label: "Email",
        value: "afkara@gmail.com",
        url: "mailto:afkara@gmail.com",
    },
    Contact {
        label: "Email",
        value: "jonas.wienstroeer@proton.me",
        url: "mailto:jonas.wienstroeer@proton.me",
    },
    Contact {
        label: "GitHub",
        value: "https://github.com/afkaralp",
        url: "https://github.com/afkaralp",
    },
    Contact {
        label: "Hugging Face",
        value: "https://huggingface.co/AfkaraLP",
        url: "https://huggingface.co/AfkaraLP",
    },
];

pub struct Host {
    pub web: &'static str,
    pub api: &'static str,
    /// JSON field of the number shown next to a project.
    pub stat: &'static str,
    pub symbol: &'static str,
}

pub const GITHUB: Host = Host {
    web: "https://github.com/",
    api: "https://api.github.com/repos/",
    stat: "stargazers_count",
    symbol: "★",
};

pub const HF_MODEL: Host = Host {
    web: "https://huggingface.co/",
    api: "https://huggingface.co/api/models/",
    stat: "downloads",
    symbol: "↓",
};

pub const HF_DATASET: Host = Host {
    web: "https://huggingface.co/datasets/",
    api: "https://huggingface.co/api/datasets/",
    stat: "downloads",
    symbol: "↓",
};

pub struct Project {
    pub name: &'static str,
    pub description: &'static str,
    pub kind: &'static str,
    pub host: &'static Host,
    pub id: &'static str,
    pub site: Option<&'static str>,
}

impl Project {
    #[must_use]
    pub fn url(&self) -> String {
        format!("{}{}", self.host.web, self.id)
    }

    #[must_use]
    pub fn api_url(&self) -> String {
        format!("{}{}", self.host.api, self.id)
    }

    #[must_use]
    pub fn parse_stat(&self, json: &str) -> Option<u32> {
        let key = format!("\"{}\":", self.host.stat);
        let rest = json[json.find(&key)? + key.len()..].trim_start();
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        rest[..end].parse().ok()
    }
}
pub const PROJECTS: &[Project] = &[
    Project {
        name: "spellsp",
        description: "Spell-checking language server (LSP) written in Rust",
        kind: "Rust",
        host: &GITHUB,
        id: "afkaralp/spellsp",
        site: None,
    },
    Project {
        name: "qwen3-tts-webui",
        description: "Web UI and API for voice cloning with Qwen3-TTS",
        kind: "Python",
        host: &GITHUB,
        id: "afkaralp/qwen3-tts-webui",
        site: None,
    },
    Project {
        name: "ascii-renderer",
        description: "2D ASCII renderer in Rust, with a 3D demo",
        kind: "Rust",
        host: &GITHUB,
        id: "afkaralp/ascii-renderer",
        site: None,
    },
    Project {
        name: "torus-http",
        description: "Tiny synchronous HTTP server library with zero dependencies",
        kind: "Rust",
        host: &GITHUB,
        id: "afkaralp/torus-http",
        site: None,
    },
    Project {
        name: "rustlean-gguf",
        description: "Rust fill-in-the-middle completion model (Qwen2.5-Coder 1.5B + LoRA). Came out of an autonomous research run on getting fast but smart code completion on my RTX 2070 Super",
        kind: "Hugging Face · model",
        host: &HF_MODEL,
        id: "AfkaraLP/rustlean-gguf",
        site: None,
    },
    Project {
        name: "CoralGPT-Supra-50M-merge-70",
        description: "50M Llama fine-tuned on Bedwars chat from the CoralMC Minecraft server",
        kind: "Hugging Face · model",
        host: &HF_MODEL,
        id: "AfkaraLP/CoralGPT-Supra-50M-merge-70",
        site: Some("https://coralgpt.afkara.dev"),
    },
    Project {
        name: "coralinsights",
        description: "866k chat messages from 30k players on CoralMC as a SQLite database, the training data for CoralGPT",
        kind: "Hugging Face · dataset",
        host: &HF_DATASET,
        id: "AfkaraLP/coralinsights",
        site: Some("https://coralinsights.afkara.dev"),
    },
    Project {
        name: "mister-president-wakeword",
        description: "OpenWakeWord model for \"mister president\", the wake word of my self-hosted voice assistant. The assistant itself is on my Forgejo and is overdue for a rewrite with newer models",
        kind: "Hugging Face · model",
        host: &HF_MODEL,
        id: "AfkaraLP/mister-president-wakeword",
        site: Some("https://forgejo.afkara.dev/hermes/hey-fable"),
    },
];

pub struct Bookmark {
    pub title: &'static str,
    pub url: &'static str,
    pub description: &'static str,
    pub tags: &'static [&'static str],
}

pub const BOOKMARKS: &[Bookmark] = &[
    Bookmark {
        title: "Attention Is All You Need",
        url: "https://arxiv.org/abs/1706.03762",
        description: "The paper that introduced the Transformer architecture",
        tags: &[
            "paper",
            "arxiv",
            "machine-learning",
            "fundamental",
            "research",
        ],
    },
    Bookmark {
        title: "The Rust Programming Language",
        url: "https://doc.rust-lang.org/book/",
        description: "The official Rust book, from the basics to advanced topics",
        tags: &["book", "rust", "programming-language", "learning"],
    },
    Bookmark {
        title: "Learn You a Haskell for Great Good!",
        url: "https://learnyouahaskell.github.io/",
        description: "A fun, beginner-friendly introduction to Haskell",
        tags: &["book", "haskell", "functional", "learning"],
    },
    Bookmark {
        title: "Auto-Encoding Variational Bayes",
        url: "https://arxiv.org/abs/1312.6114",
        description: "Kingma & Welling introduce the Variational Autoencoder",
        tags: &[
            "paper",
            "arxiv",
            "machine-learning",
            "deep-learning",
            "generative",
            "research",
        ],
    },
    Bookmark {
        title: "Neural Discrete Representation Learning",
        url: "https://arxiv.org/abs/1711.00937",
        description: "van den Oord et al. introduce VQ-VAE: discrete latents via vector quantization",
        tags: &[
            "paper",
            "arxiv",
            "machine-learning",
            "deep-learning",
            "representation-learning",
            "research",
        ],
    },
    Bookmark {
        title: "Stable Audio 3",
        url: "https://arxiv.org/abs/2605.17991",
        description: "Stability AI's diffusion model for text-to-audio generation",
        tags: &[
            "paper",
            "arxiv",
            "machine-learning",
            "audio",
            "generative",
            "research",
        ],
    },
    Bookmark {
        title: "SAME: A Semantically-Aligned Music Autoencoder",
        url: "https://arxiv.org/abs/2605.18613",
        description: "A music autoencoder whose latent space is aligned with semantic structure",
        tags: &[
            "paper",
            "arxiv",
            "machine-learning",
            "audio",
            "music",
            "research",
        ],
    },
    Bookmark {
        title: "Neural Networks Are Already Clean",
        url: "https://already-clean.goodfire.com/",
        description: "Goodfire's working hypothesis about neurons, generalization and the ecology of circuits",
        tags: &[
            "article",
            "goodfire",
            "interpretability",
            "machine-learning",
            "research",
        ],
    },
    Bookmark {
        title: "DeepSeek-V4.1-Flash: Pushing the Limits of KV Cache Compression",
        url: "https://www.alphaxiv.org/pdf/2609.19969",
        description: "552B MoE with a causal encoder-decoder, cross-layer KV cache reuse and FP4 KV caching, getting the KV cache down to 890 bytes per token",
        tags: &[
            "paper",
            "alphaxiv",
            "machine-learning",
            "llm",
            "inference",
            "research",
        ],
    },
    Bookmark {
        title: "The Ultimate MoE Training Handbook",
        url: "https://huggingface.co/spaces/huggingface/moe-blogpost",
        description: "A hands-on guide to training Mixture-of-Experts models: the router, load balancing, experts, block layout, training and systems, setting by setting",
        tags: &[
            "article",
            "huggingface",
            "machine-learning",
            "moe",
            "training",
            "research",
        ],
    },
];

/// Civil year from a unix timestamp (Howard Hinnant's `civil_from_days`).
#[must_use]
pub fn year_from_unix(secs: i64) -> u32 {
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let year = yoe + era * 400 + i64::from(mp >= 10);
    u32::try_from(year).unwrap_or_default()
}

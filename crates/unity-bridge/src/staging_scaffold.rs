//! Embedded staging scaffold: the VUA Bridge package plus a minimal
//! Modular Avatar compile stub, written into every bundled staging project.
//!
//! Without these the staging project cannot execute a single Bridge
//! command — the batchmode entry point does not exist, and the Bridge's
//! asmdef references `nadena.dev.modular-avatar.core`, so neither the
//! Bridge nor imported outfit scripts compile. The real-Unity harness
//! proved exactly this scaffold (Bridge + stub) is sufficient; the bundled
//! template now ships the same thing by default.
//!
//! Single source: the Bridge files are embedded BY REFERENCE from the
//! versioned package at `unity/Packages/com.ph-r.vua` — editing the C#
//! there automatically updates the scaffold at compile time. Adding a file
//! to the package requires adding its row here.

/// The staging project is a conversion environment, not a user project: the
/// stub satisfies compilation of MA-typed references inside staging only.
/// It is VUA-authored synthetic code with no third-party content.
pub const MA_STUB_PACKAGE_ID: &str = "nadena.dev.modular-avatar.core";

/// Candidate N3 files require the real MA package. They are deliberately
/// excluded from conversion scaffolds that use the minimal compile stub.
pub static AMF_PRODUCTION_BRIDGE_FILES: &[(&str, &str)] = &[
    ("Editor/Amf.meta", include_str!("../../../unity/Packages/com.ph-r.vua/Editor/Amf.meta")),
    ("Editor/Amf/AmfProductionEntryPoint.cs", include_str!("../../../unity/Packages/com.ph-r.vua/Editor/Amf/AmfProductionEntryPoint.cs")),
    ("Editor/Amf/AmfProductionEntryPoint.cs.meta", include_str!("../../../unity/Packages/com.ph-r.vua/Editor/Amf/AmfProductionEntryPoint.cs.meta")),
    ("Editor/Amf/AmfProductionEntryPoint.Release.cs", include_str!("../../../unity/Packages/com.ph-r.vua/Editor/Amf/AmfProductionEntryPoint.Release.cs")),
    ("Editor/Amf/AmfProductionEntryPoint.Release.cs.meta", include_str!("../../../unity/Packages/com.ph-r.vua/Editor/Amf/AmfProductionEntryPoint.Release.cs.meta")),
];

pub const MA_STUB_PACKAGE_JSON: &str =
    r#"{ "name": "nadena.dev.modular-avatar.core", "version": "0.0.0-stub" }"#;

pub const MA_STUB_ASMDEF: &str =
    r#"{ "name": "nadena.dev.modular-avatar.core", "rootNamespace": "" }"#;

pub const MA_STUB_COMPONENTS_CS: &str = r#"using System.Collections.Generic;
using UnityEngine;

namespace nadena.dev.modular_avatar.core
{
    public class AvatarObjectReference
    {
        public AvatarObjectReference() { }
        public AvatarObjectReference(GameObject target) { }
    }

    public enum PortableControlType { Menu, Toggle, Submenu, Action }

    public class PortableControl
    {
        public PortableControlType Type;
        public int Value;
    }

    public class ToggledObject
    {
        public AvatarObjectReference Object;
        public bool Active;
    }

    public class ModularAvatarMergeArmature : MonoBehaviour
    {
        public AvatarObjectReference mergeTarget;
        public void InferPrefixSuffix() { }
    }

    public class ModularAvatarMenuInstaller : MonoBehaviour { }

    public class ModularAvatarMenuItem : MonoBehaviour
    {
        public string label;
        public PortableControl PortableControl = new PortableControl();
        public bool isDefault;
    }

    public class ModularAvatarObjectToggle : MonoBehaviour
    {
        public List<ToggledObject> Objects;
    }
}
"#;

/// The VUA Bridge package, embedded by reference: (package-relative path,
/// contents). Everything the package needs to compile and expose the
/// batchmode entry point — including `.meta` files so import is stable.
pub static BRIDGE_PACKAGE_FILES: &[(&str, &str)] = &[
    (
        "package.json",
        include_str!("../../../unity/Packages/com.ph-r.vua/package.json"),
    ),
    (
        "package.json.meta",
        include_str!("../../../unity/Packages/com.ph-r.vua/package.json.meta"),
    ),
    (
        "Editor.meta",
        include_str!("../../../unity/Packages/com.ph-r.vua/Editor.meta"),
    ),
    (
        "Editor/AssemblyInfo.cs",
        include_str!("../../../unity/Packages/com.ph-r.vua/Editor/AssemblyInfo.cs"),
    ),
    (
        "Editor/AssemblyInfo.cs.meta",
        include_str!("../../../unity/Packages/com.ph-r.vua/Editor/AssemblyInfo.cs.meta"),
    ),
    (
        "Editor/Vua.Editor.asmdef",
        include_str!("../../../unity/Packages/com.ph-r.vua/Editor/Vua.Editor.asmdef"),
    ),
    (
        "Editor/Vua.Editor.asmdef.meta",
        include_str!("../../../unity/Packages/com.ph-r.vua/Editor/Vua.Editor.asmdef.meta"),
    ),
    (
        "Editor/Bridge.meta",
        include_str!("../../../unity/Packages/com.ph-r.vua/Editor/Bridge.meta"),
    ),
    (
        "Editor/Bridge/BridgeCommandProcessor.cs",
        include_str!("../../../unity/Packages/com.ph-r.vua/Editor/Bridge/BridgeCommandProcessor.cs"),
    ),
    (
        "Editor/Bridge/BridgeCommandProcessor.cs.meta",
        include_str!(
            "../../../unity/Packages/com.ph-r.vua/Editor/Bridge/BridgeCommandProcessor.cs.meta"
        ),
    ),
    (
        "Editor/Bridge/BridgeEntryPoint.cs",
        include_str!("../../../unity/Packages/com.ph-r.vua/Editor/Bridge/BridgeEntryPoint.cs"),
    ),
    (
        "Editor/Bridge/BridgeEntryPoint.cs.meta",
        include_str!("../../../unity/Packages/com.ph-r.vua/Editor/Bridge/BridgeEntryPoint.cs.meta"),
    ),
    (
        "Editor/Bridge/BridgeProtocol.cs",
        include_str!("../../../unity/Packages/com.ph-r.vua/Editor/Bridge/BridgeProtocol.cs"),
    ),
    (
        "Editor/Bridge/BridgeProtocol.cs.meta",
        include_str!("../../../unity/Packages/com.ph-r.vua/Editor/Bridge/BridgeProtocol.cs.meta"),
    ),
    (
        "Editor/Bridge/BridgePreviewBake.cs",
        include_str!("../../../unity/Packages/com.ph-r.vua/Editor/Bridge/BridgePreviewBake.cs"),
    ),
    (
        "Editor/Bridge/BridgePreviewBake.cs.meta",
        include_str!("../../../unity/Packages/com.ph-r.vua/Editor/Bridge/BridgePreviewBake.cs.meta"),
    ),
    (
        "Editor/Bridge/BridgeResultJson.cs",
        include_str!("../../../unity/Packages/com.ph-r.vua/Editor/Bridge/BridgeResultJson.cs"),
    ),
    (
        "Editor/Bridge/BridgeResultJson.cs.meta",
        include_str!("../../../unity/Packages/com.ph-r.vua/Editor/Bridge/BridgeResultJson.cs.meta"),
    ),
];

/// Writes the embedded scaffold (Bridge package + MA compile stub) into
/// `packages_root` (the staging project's `Packages/` directory).
pub fn write_scaffold(packages_root: &std::path::Path) -> std::io::Result<()> {
    let bridge_root = packages_root.join(crate::material_staging::BRIDGE_PACKAGE_ID);
    for (relative, contents) in BRIDGE_PACKAGE_FILES {
        let target = bridge_root.join(relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, contents)?;
    }

    let stub_root = packages_root.join(MA_STUB_PACKAGE_ID);
    std::fs::create_dir_all(&stub_root)?;
    std::fs::write(stub_root.join("package.json"), MA_STUB_PACKAGE_JSON)?;
    std::fs::write(
        stub_root.join(format!("{MA_STUB_PACKAGE_ID}.asmdef")),
        MA_STUB_ASMDEF,
    )?;
    std::fs::write(stub_root.join("StubComponents.cs"), MA_STUB_COMPONENTS_CS)?;
    Ok(())
}

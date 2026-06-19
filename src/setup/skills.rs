use anyhow::{Context, Result};
use std::path::Path;

const SENSEZ_SKILL: &str = include_str!("../../skills/sensez/SKILL.md");

pub fn install(root: &Path, agent: &str) -> Result<Option<String>> {
    let Some(rel) = crate::setup::agents::find(agent).and_then(|spec| spec.skill_relpath) else {
        return Ok(None);
    };
    let source = root.join("skills").join("sensez");
    let dest = root.join(rel);
    std::fs::create_dir_all(&dest).with_context(|| format!("creating {}", dest.display()))?;
    if source.join("SKILL.md").exists() {
        copy_dir(&source, &dest)?;
    } else {
        let skill = dest.join("SKILL.md");
        std::fs::write(&skill, SENSEZ_SKILL)
            .with_context(|| format!("writing {}", skill.display()))?;
    }
    Ok(Some(format!(
        "installed Sensez skill in {}",
        dest.display()
    )))
}

fn copy_dir(source: &Path, dest: &Path) -> Result<()> {
    for entry in
        std::fs::read_dir(source).with_context(|| format!("reading {}", source.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        let target = dest.join(entry.file_name());
        if path.is_dir() {
            std::fs::create_dir_all(&target)
                .with_context(|| format!("creating {}", target.display()))?;
            copy_dir(&path, &target)?;
        } else if path.is_file() {
            std::fs::copy(&path, &target)
                .with_context(|| format!("copying {} to {}", path.display(), target.display()))?;
        }
    }
    Ok(())
}

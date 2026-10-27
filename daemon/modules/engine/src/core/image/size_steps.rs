use crate::prelude::*;

/// サーバが画像の幅と高さを切り上げる先の、固定値の集合。
///
/// クライアントが画面の幅や画素密度から計算した値をそのまま受けると、ほぼ同じ大きさの変換結果が値の数だけキャッシュに並ぶ。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SizeSteps(Vec<u32>);

impl SizeSteps {
    /// 空の集合と 0 を含む集合は拒否する。並びと重複は整える。
    pub fn new(mut steps: Vec<u32>) -> Result<Self> {
        if steps.is_empty() || steps.contains(&0) {
            return Err(Error::new(ErrorKind::InvalidInput).with_message("size steps must be non-empty and greater than zero"));
        }
        steps.sort_unstable();
        steps.dedup();
        Ok(Self(steps))
    }

    /// 要求以上の最小の段階を返す。最大の段階を超える要求は、最大の段階に丸める。
    pub fn round_up(&self, value: u32) -> u32 {
        self.0.iter().copied().find(|step| *step >= value).unwrap_or_else(|| self.max())
    }

    /// The first `count` steps in ascending order, or all of them when there are fewer.
    pub fn smallest(&self, count: usize) -> &[u32] {
        &self.0[..count.min(self.0.len())]
    }

    pub fn max(&self) -> u32 {
        self.0.last().copied().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use testresult::TestResult;

    use super::*;

    #[test]
    fn rounds_up_to_the_smallest_step_test() -> TestResult {
        let steps = SizeSteps::new(vec![512, 128, 1024, 256, 256])?;
        assert_eq!(steps.round_up(1), 128);
        assert_eq!(steps.round_up(128), 128);
        assert_eq!(steps.round_up(129), 256);
        assert_eq!(steps.round_up(300), 512);
        assert_eq!(steps.round_up(1024), 1024);
        assert_eq!(steps.round_up(2000), 1024, "最大の段階を超える要求は最大の段階に丸める");
        Ok(())
    }

    #[test]
    fn smallest_returns_the_first_steps_in_order_test() -> TestResult {
        let steps = SizeSteps::new(vec![512, 128, 1024, 256])?;
        assert_eq!(steps.smallest(3), [128, 256, 512]);
        assert_eq!(steps.smallest(0), [] as [u32; 0]);
        assert_eq!(steps.smallest(9), [128, 256, 512, 1024]);
        assert_eq!(SizeSteps::new(vec![256, 64])?.smallest(3), [64, 256], "a set with fewer steps returns all of them");
        Ok(())
    }

    #[test]
    fn rejects_empty_and_zero_steps_test() {
        assert_eq!(*SizeSteps::new(vec![]).unwrap_err().kind(), ErrorKind::InvalidInput);
        assert_eq!(*SizeSteps::new(vec![128, 0]).unwrap_err().kind(), ErrorKind::InvalidInput);
    }
}

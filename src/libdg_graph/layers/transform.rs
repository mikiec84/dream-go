// Copyright 2019 Karl Sundequist Blomdahl <karl.sundequist.blomdahl@gmail.com>
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use libc::c_void;

use ::graph_def::LayerDef;
use ::layer::{Layer, PreparedLayer};
use dg_cuda as cuda;
use dg_cuda::cudnn;

#[derive(Clone, Debug)]
pub struct Transform;

impl Layer for Transform {
    fn prepare(
        &self,
        _handle: &cudnn::Handle,
        _inputs: &[&cudnn::Tensor],
        _outputs: &[&cudnn::Tensor]
    ) -> Result<Box<PreparedLayer>, cuda::Error>
    {
        Ok(Box::new(self.clone()))
    }
}

impl PreparedLayer for Transform {
    fn size_in_bytes(&self) -> usize {
        0
    }

    fn forward(
        &self,
        handle: &cudnn::Handle,
        inputs: &[(&cudnn::Tensor, *const c_void)],
        outputs: &[(&cudnn::Tensor, *mut c_void)],
        _workspace_ptr: *mut c_void
    ) -> Result<(), cuda::Error>
    {
        assert_eq!(inputs.len(), 1);
        assert_eq!(outputs.len(), 1);

        let transform = cudnn::Transform::new()?;

        transform.forward(
            handle,
            inputs[0].0,
            inputs[0].1,
            outputs[0].0,
            outputs[0].1
        )
    }
}

impl Transform {
    pub fn new(_layer_def: &LayerDef) -> Result<Transform, cuda::Error> {
        Ok(Transform)
    }
}

#[cfg(test)]
mod tests {
    use dg_utils::types::f16;
    use graph_def::{DataTypeDef, LayerDef, LayerTypeDef, VariableDef};
    use layers::tests::{assert_approx_eq, run_layer};
    use layers::Transform;

    fn check_transform<I, O>(input_type: DataTypeDef, output_type: DataTypeDef)
    where I: From<f32> + Clone + Copy + Default,
          O: From<f32> + Clone + Copy + Default,
          f32: From<I>,
          f32: From<O>
    {
        let layer_def = LayerDef {
            type_of: LayerTypeDef::Scale,
            input: vec! [
                VariableDef { id: 0, shape: vec! [1, 19, 19, 64], data_type: input_type }
            ],
            output: vec! [
                VariableDef { id: 0, shape: vec! [1, 19, 19, 64], data_type: output_type }
            ],
            arguments: None
        };
        let layer = Transform::new(&layer_def)
            .expect("Could not create scale layer");

        let (inputs, outputs) = run_layer::<I, O, _>(
            &layer_def,
            &layer
        );

        for (&inp, &outp) in inputs[0].iter().zip(outputs[0].iter()) {
            assert_approx_eq(f32::from(inp), f32::from(outp));
        }
    }

    #[test]
    fn half_to_float() {
        check_transform::<f16, f32>(DataTypeDef::Half, DataTypeDef::Float);
    }

    #[test]
    fn half_to_half() {
        check_transform::<f16, f16>(DataTypeDef::Half, DataTypeDef::Half);
    }

    #[test]
    fn float_to_half() {
        check_transform::<f32, f16>(DataTypeDef::Float, DataTypeDef::Half);
    }

    #[test]
    fn float_to_float() {
        check_transform::<f32, f32>(DataTypeDef::Float, DataTypeDef::Float);
    }
}
use spirv_std::glam::{vec4, Vec4};

use crate::{AlphaMode, BvhNode, Materials, Params};

pub fn run<P>(materials: &Materials<P>, nodes: &[BvhNode], out: &mut Vec<Vec4>)
where
    P: Params,
{
    run_ex(materials, nodes, out, 0);
}

fn run_ex<P>(
    materials: &Materials<P>,
    nodes: &[BvhNode],
    out: &mut Vec<Vec4>,
    node_id: u32,
) -> u32
where
    P: Params,
{
    const OP_INTERNAL: u32 = 0;
    const OP_LEAF: u32 = 1;

    let ptr = out.len();

    match &nodes[node_id as usize] {
        BvhNode::Internal { left_node_id, .. } => {
            out.push(Default::default());
            out.push(Default::default());
            out.push(Default::default());
            out.push(Default::default());

            let left_node_id = *left_node_id;
            let right_node_id = left_node_id + 1;

            let left_bb = nodes[left_node_id as usize].bounds();
            let right_bb = nodes[right_node_id as usize].bounds();

            let _left_ptr = run_ex(materials, nodes, out, left_node_id);
            let right_ptr = run_ex(materials, nodes, out, right_node_id);

            out[ptr] = vec4(
                left_bb.min().x,
                left_bb.min().y,
                left_bb.min().z,
                f32::from_bits(OP_INTERNAL),
            );

            out[ptr + 1] = vec4(
                left_bb.max().x,
                left_bb.max().y,
                left_bb.max().z,
                f32::from_bits(right_ptr),
            );

            // TODO we could store information about transparency here to
            //      quickly reject nodes during bvh traversal later
            out[ptr + 2] = vec4(
                right_bb.min().x,
                right_bb.min().y,
                right_bb.min().z,
                Default::default(),
            );

            out[ptr + 3] = vec4(
                right_bb.max().x,
                right_bb.max().y,
                right_bb.max().z,
                Default::default(),
            );
        }

        BvhNode::Leaf { primitives, .. } => {
            for (primitive_idx, primitive) in primitives.iter().enumerate() {
                let material = &materials[primitive.material_id];

                let flags = {
                    let got_more_triangles =
                        primitive_idx + 1 < primitives.len();

                    let has_alpha_blending =
                        matches!(material.alpha_mode, AlphaMode::Blend);

                    (got_more_triangles as u32)
                        | ((has_alpha_blending as u32) << 1)
                };

                out.push(vec4(
                    f32::from_bits(flags),
                    f32::from_bits(primitive.triangle_id.get()),
                    f32::from_bits(primitive.material_id.get()),
                    f32::from_bits(OP_LEAF),
                ));
            }
        }
    }

    ptr as u32
}

fn trace_first(pos_view: vec4<f32>, dir_view: vec4<f32>) -> vec4<f32> {
    var depth = 9999999999999.0;
    var result = vec4<f32>(0.0);

    var group: VoxelGroup;
    var dim_f: vec3<f32>;
    var dim_i: vec3<i32>;
    var pos: vec3<f32>;
    var dir: vec3<f32>;
    var dir_if: vec3<f32>;

    var plane_point: vec3<f32>;
    var t_offset: f32;
    var t_i: vec3<f32>;
    var px: vec3<f32>;
    var py: vec3<f32>;
    var pz: vec3<f32>;
    var hit: vec3<bool>;
    var vox_pos: vec3<i32>;

    var dir_i: vec3<i32>;
    var inc_t: vec3<f32>;
    var corner: vec3<f32>;
    var next_t: vec3<f32>;
    var t: f32;
    var prev_t: f32;
    var color: vec4<f32>;
    var gdepth: f32;
    var i: u32;
    var vcolor: vec4<f32>;
    var axis: i32;
    var full_t: f32;

    for (var gi: u32 = 0; gi < arrayLength(&voxel_groups); gi = gi + 1) {
        group = voxel_groups[gi];
        dim_f = vec3<f32>(group.dimensions);
        dim_i = vec3<i32>(group.dimensions);

        // transform so that group is at 0,0
        pos = (group.transform * pos_view).xyz;
        dir = (group.transform * dir_view).xyz;

        dir_if = sign(dir);



        // find where ray intersects with group
        plane_point = (vec3<f32>(1.0) - dir_if) / 2.0 * dim_f;
        t_offset = 0.0;
        if outside3f(pos, ZERO3F, dim_f) {
            // time of intersection; x = td + p, solve for t
            t_i = (plane_point - pos) / dir;
            // points of intersection
            px = pos + t_i.x * dir;
            py = pos + t_i.y * dir;
            pz = pos + t_i.z * dir;

            // check if point is in bounds
            hit = vec3<bool>(
                inside2f(px.yz, ZERO2F, dim_f.yz),
                inside2f(py.xz, ZERO2F, dim_f.xz),
                inside2f(pz.xy, ZERO2F, dim_f.xy),
            ) && (t_i > ZERO3F);
            if !any(hit) {
                continue;
            }
            pos = select(select(pz, py, hit.y), px, hit.x);
            t_offset = select(select(t_i.z, t_i.y, hit.y), t_i.x, hit.x);
        }
        vox_pos = clamp(vec3<i32>(pos), vec3<i32>(0), dim_i - vec3<i32>(1));



        dir_i = vec3<i32>(dir_if);
        // time to move 1 unit using dir
        inc_t = abs(1.0 / dir);
        corner = vec3<f32>(vox_pos) + vec3<f32>(0.5) + dir_if / 2.0;

        // time of next plane hit for each direction
        next_t = inc_t * abs(pos - corner);
        t = 0.0;
        prev_t = t;
        color = vec4<f32>(0.0);
        gdepth = 9999999999999.0;
        loop {
            i = u32(vox_pos.x + vox_pos.y * dim_i.x + vox_pos.z * dim_i.x * dim_i.y) + group.offset;
            vcolor = unpack4x8unorm(voxels[i]);

            // select next voxel to move to next based on least time
            axis = select(select(2, 1, next_t.y < next_t.z), 0, next_t.x < next_t.y && next_t.x < next_t.z);
            prev_t = t;
            t = next_t[axis];
            vox_pos[axis] += dir_i[axis];
            next_t[axis] += inc_t[axis];

            // hit a voxel
            if vcolor.a > 0.0 {
                full_t = t_offset + prev_t;
                gdepth = min(gdepth, full_t);
                color += vec4<f32>(vcolor.xyz * vcolor.a * (1.0 - color.a), (1.0 - color.a) * vcolor.a);
                if color.a >= FULL_ALPHA {
                    break;
                }
            }

            if color.a >= FULL_ALPHA || vox_pos[axis] < 0 || vox_pos[axis] >= dim_i[axis] {
                break;
            }
        }
        result = select(result, color, gdepth < depth);
        depth = min(gdepth, depth);
    }
    return result;
}


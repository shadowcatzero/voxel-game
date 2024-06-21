fn trace_one(gi: u32, pos_view: vec4<f32>, dir_view: vec4<f32>) -> vec4<f32> {
    let group = voxel_groups[gi];
    let dim_f = vec3<f32>(group.dimensions);
    let dim_i = vec3<i32>(group.dimensions);

    // transform so that group is at 0,0
    var pos = (group.transform * pos_view).xyz;
    let dir = (group.transform * dir_view).xyz;

    let dir_if = sign(dir);



    // find where ray intersects with group
    let plane_point = (vec3<f32>(1.0) - dir_if) / 2.0 * dim_f;
    var t_offset = 0.0;
    if outside3f(pos, ZERO3F, dim_f) {
        // time of intersection; x = td + p, solve for t
        let t_i = (plane_point - pos) / dir;
        // points of intersection
        let px = pos + t_i.x * dir;
        let py = pos + t_i.y * dir;
        let pz = pos + t_i.z * dir;

        // check if point is in bounds
        let hit = vec3<bool>(
            inside2f(px.yz, ZERO2F, dim_f.yz),
            inside2f(py.xz, ZERO2F, dim_f.xz),
            inside2f(pz.xy, ZERO2F, dim_f.xy),
        ) && (t_i > ZERO3F);
        if !any(hit) {
            return vec4<f32>(0.0);
        }
        pos = select(select(pz, py, hit.y), px, hit.x);
        t_offset = select(select(t_i.z, t_i.y, hit.y), t_i.x, hit.x);
    }
    var vox_pos = clamp(vec3<i32>(pos), vec3<i32>(0), dim_i - vec3<i32>(1));



    let dir_i = vec3<i32>(dir_if);
    // time to move 1 unit using dir
    let inc_t = abs(1.0 / dir);
    let corner = vec3<f32>(vox_pos) + vec3<f32>(0.5) + dir_if / 2.0;

    // time of next plane hit for each direction
    var next_t = inc_t * abs(pos - corner);
    var color = vec4<f32>(0.0);
    loop {
        let i = u32(vox_pos.x + vox_pos.y * dim_i.x + vox_pos.z * dim_i.x * dim_i.y) + group.offset;
        var vcolor = unpack4x8unorm(voxels[i]);

        // select next voxel to move to next based on least time
        let axis = select(select(2, 1, next_t.y < next_t.z), 0, next_t.x < next_t.y && next_t.x < next_t.z);
        vox_pos[axis] += dir_i[axis];
        next_t[axis] += inc_t[axis];
        color += vec4<f32>(vcolor.xyz * vcolor.a * (1.0 - color.a), (1.0 - color.a) * vcolor.a);

        if color.a >= FULL_ALPHA || vox_pos[axis] < 0 || vox_pos[axis] >= dim_i[axis] {
            break;
        }
    }
    return color;
}

fn trace_opaque(pos_view: vec4<f32>, dir_view: vec4<f32>) -> vec4<f32> {
    var depth = 9999999999999.0;
    var result = vec4<f32>(0.0);
    for (var gi: u32 = 0; gi < arrayLength(&voxel_groups); gi = gi + 1) {
        let group = voxel_groups[gi];
        let dim_f = vec3<f32>(group.dimensions);
        let dim_i = vec3<i32>(group.dimensions);

        // transform so that group is at 0,0
        var pos = (group.transform * pos_view).xyz;
        let dir = (group.transform * dir_view).xyz;

        let dir_if = sign(dir);



        // find where ray intersects with group
        let plane_point = (vec3<f32>(1.0) - dir_if) / 2.0 * dim_f;
        var t_offset = 0.0;
        if outside3f(pos, ZERO3F, dim_f) {
            // time of intersection; x = td + p, solve for t
            let t_i = (plane_point - pos) / dir;
            // points of intersection
            let px = pos + t_i.x * dir;
            let py = pos + t_i.y * dir;
            let pz = pos + t_i.z * dir;

            // check if point is in bounds
            let hit = vec3<bool>(
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
        var vox_pos = clamp(vec3<i32>(pos), vec3<i32>(0), dim_i - vec3<i32>(1));



        let dir_i = vec3<i32>(dir_if);
        // time to move 1 unit using dir
        let inc_t = abs(1.0 / dir);
        let corner = vec3<f32>(vox_pos) + vec3<f32>(0.5) + dir_if / 2.0;

        // time of next plane hit for each direction
        var next_t = inc_t * abs(pos - corner);
        var t = 0.0;
        var prev_t = t;
        var color = vec4<f32>(0.0);
        var gdepth = 9999999999999.0;
        loop {
            let i = u32(vox_pos.x + vox_pos.y * dim_i.x + vox_pos.z * dim_i.x * dim_i.y) + group.offset;
            var vcolor = unpack4x8unorm(voxels[i]);

            // select next voxel to move to next based on least time
            let axis = select(select(2, 1, next_t.y < next_t.z), 0, next_t.x < next_t.y && next_t.x < next_t.z);
            prev_t = t;
            t = next_t[axis];
            vox_pos[axis] += dir_i[axis];
            next_t[axis] += inc_t[axis];

            // hit a voxel
            if vcolor.a > 0.0 {
                let full_t = t_offset + prev_t;
                gdepth = min(gdepth, full_t);
                color = vcolor;
                break;
            }

            if vox_pos[axis] < 0 || vox_pos[axis] >= dim_i[axis] {
                break;
            }
        }
        result = select(result, color, gdepth < depth);
        depth = min(gdepth, depth);
    }
    return result;
}

fn trace_first(pos_view: vec4<f32>, dir_view: vec4<f32>) -> vec4<f32> {
    var depth = 9999999999999.0;
    var result = vec4<f32>(0.0);
    for (var gi: u32 = 0; gi < arrayLength(&voxel_groups); gi = gi + 1) {
        let group = voxel_groups[gi];
        let dim_f = vec3<f32>(group.dimensions);
        let dim_i = vec3<i32>(group.dimensions);

        // transform so that group is at 0,0
        var pos = (group.transform * pos_view).xyz;
        let dir = (group.transform * dir_view).xyz;

        let dir_if = sign(dir);



        // find where ray intersects with group
        let plane_point = (vec3<f32>(1.0) - dir_if) / 2.0 * dim_f;
        var t_offset = 0.0;
        if outside3f(pos, ZERO3F, dim_f) {
            // time of intersection; x = td + p, solve for t
            let t_i = (plane_point - pos) / dir;
            // points of intersection
            let px = pos + t_i.x * dir;
            let py = pos + t_i.y * dir;
            let pz = pos + t_i.z * dir;

            // check if point is in bounds
            let hit = vec3<bool>(
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
        var vox_pos = clamp(vec3<i32>(pos), vec3<i32>(0), dim_i - vec3<i32>(1));



        let dir_i = vec3<i32>(dir_if);
        // time to move 1 unit using dir
        let inc_t = abs(1.0 / dir);
        let corner = vec3<f32>(vox_pos) + vec3<f32>(0.5) + dir_if / 2.0;

        // time of next plane hit for each direction
        var next_t = inc_t * abs(pos - corner);
        var t = 0.0;
        var prev_t = t;
        var color = vec4<f32>(0.0);
        var gdepth = 9999999999999.0;
        loop {
            let i = u32(vox_pos.x + vox_pos.y * dim_i.x + vox_pos.z * dim_i.x * dim_i.y) + group.offset;
            var vcolor = unpack4x8unorm(voxels[i]);

            // select next voxel to move to next based on least time
            let axis = select(select(2, 1, next_t.y < next_t.z), 0, next_t.x < next_t.y && next_t.x < next_t.z);
            prev_t = t;
            t = next_t[axis];
            vox_pos[axis] += dir_i[axis];
            next_t[axis] += inc_t[axis];

            // hit a voxel
            if vcolor.a > 0.0 {
                let full_t = t_offset + prev_t;
                gdepth = min(gdepth, full_t);
                color += vec4<f32>(vcolor.xyz * vcolor.a * (1.0 - color.a), (1.0 - color.a) * vcolor.a);
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


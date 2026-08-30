// Word Clock 结构件 - 参数化模型
// 用法: 修改 part 选择导出件, F6 渲染后 File > Export > Export as STL
part = "grid"; // ["grid", "backplate", "faceplate_frame", "all"]

// ===== 核心参数（与硬件绑定）=====
pitch      = 16.67; // 灯距: 60灯/米（实测后再定稿）
cols       = 11;
rows       = 10;
grid_wall  = 1.2;   // 格栅壁厚（3×0.4 挤出）
grid_h     = 15;    // 格栅高度（防串光，≥12 推荐）
margin     = 10;    // 字面外边框
plate_t    = 2.4;   // 板厚
strip_w    = 10.3;  // LED 槽宽 = 灯条 PCB 宽 + 0.3 公差

grid_w = cols * pitch;  // 183.4
grid_d = rows * pitch;  // 166.7

echo(str("字面: ", grid_w, " x ", grid_d, " mm"));
echo(str("整机: ", grid_w + 2*margin, " x ", grid_d + 2*margin, " mm"));

// ===== 格栅：每字一格 =====
module grid() {
    // 纵墙
    for (i = [0:cols])
        translate([i * pitch - grid_wall / 2, 0, 0])
            cube([grid_wall, grid_d, grid_h]);
    // 横墙
    for (j = [0:rows])
        translate([0, j * pitch - grid_wall / 2, 0])
            cube([grid_w, grid_wall, grid_h]);
}

// ===== 背板：含 LED 行定位槽与走线孔 =====
module backplate() {
    difference() {
        cube([grid_w + 2 * margin, grid_d + 2 * margin, plate_t]);
        // LED 行槽（每行一条，槽底面贴灯条）
        for (j = [0:rows - 1])
            translate([margin, margin + j * pitch + (pitch - strip_w) / 2, plate_t - 1.2])
                cube([grid_w, strip_w, 1.2 + 0.01]);
        // 行端走线孔（蛇形接线过孔）
        for (j = [0:rows - 1])
            translate([margin + grid_w - 3, margin + j * pitch + pitch / 2 - 2, -0.01])
                cube([6, 4, plate_t + 0.02]);
    }
}

// ===== 前面板框（字母镂空另行处理，见 README 方案 A/B/C）=====
module faceplate_frame() {
    difference() {
        cube([grid_w + 2 * margin, grid_d + 2 * margin, plate_t]);
        // 字面开窗：后续在此叠加字母镂空（stencil 字体）或整体换亚克力+车贴
        translate([margin, margin, -0.01])
            cube([grid_w, grid_d, plate_t + 0.02]);
    }
}

// ===== 角点分钟 LED 位置参考（四角各 1 颗）=====
module corner_dot_markers() {
    for (x = [margin / 2, grid_w + 1.5 * margin])
        for (y = [margin / 2, grid_d + 1.5 * margin])
            translate([x, y, 0]) cylinder(d = 5, h = 2, $fn = 24);
}

if (part == "grid")              translate([-margin, -margin, 0]) grid();
if (part == "backplate")         backplate();
if (part == "faceplate_frame")   faceplate_frame();
if (part == "all") {
    backplate();
    translate([0, 0, plate_t]) translate([0, 0, 0]) grid_at_margin();
    translate([0, 0, plate_t + grid_h + 6]) faceplate_frame();
}
module grid_at_margin() { translate([margin, margin, 0]) grid(); }

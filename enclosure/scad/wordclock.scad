// Word Clock 结构件 - 参数化模型 v2
// 坐标约定: 原点=背板左下角, +X=列方向(11), +Y=行方向(10), +Z=朝外出光
// 用法: 修改 part 选择导出件, F6 渲染后 File > Export > Export as STL
part = "grid"; // ["grid", "backplate", "faceplate", "corner_ring", "pitch_jig", "all"]

// ===== 核心参数（与硬件绑定，改动前先读 enclosure/README.md）=====
pitch      = 16.58; // ★灯距: 卡尺实测值。定稿前必须用"跨度法"复核（见 README）:
                    //   16.58 与 60灯/米标称 16.667 在 11 列方向累积差约 1mm,
                    //   直接决定格栅能否对齐, 先打 pitch_jig 验证再出全尺寸件
cols       = 11;
rows       = 10;
grid_wall  = 1.2;   // 格栅壁厚（0.4 喷嘴 3 圈挤出）
grid_h     = 15;    // 格栅高度 = 灯珠到扩散层距离（防串光 & 匀光, ≥12）
margin     = 14;    // 字面外边框（加宽以容纳四角分钟点腔 + 螺丝孔）
plate_t    = 2.4;   // 背板 / 前面板厚
strip_w    = 10.3;  // 行槽宽 = 灯条 PCB 实宽 + 0.3（实测后修正）
strip_t    = 1.0;   // 行槽深: PCB+背胶沉入背板, 灯珠露在槽上方
chan_clear = 0.3;   // 行槽端部余隙（x 向定位靠目视对齐, 见 README 装配）
notch_h    = 2.8;   // 纵墙过条缺口高: 需 > 槽深内净高+灯珠高(≈2.1), 留余量
notch_w    = strip_w + 0.5; // 纵墙过条缺口宽（容纳灯条穿过）

dot_id     = 9;     // 四角分钟点光腔内径
dot_wall   = 2;     // 光腔壁厚
dot_win    = 7;     // 前面板分钟点圆窗直径
post_d     = 9;     // 格栅四边中点螺丝柱径
screw_d    = 3.4;   // M3 过孔
pilot_d    = 2.8;   // M3 自攻底孔（PLA 内螺纹, 少拆装场景够用）

// ===== 派生尺寸 =====
grid_w  = cols * pitch;         // ≈182.4
grid_d  = rows * pitch;         // ≈165.8
face_w  = grid_w + 2 * margin;  // ≈210.4
face_d  = grid_d + 2 * margin;  // ≈193.8
dot_off = margin / 2;           // 分钟点圆心距板边 ≈7

echo(str("pitch=", pitch, "  字面 ", grid_w, " x ", grid_d,
         "  整机 ", face_w, " x ", face_d, " (250 热床 OK)"));

// 夹紧螺丝点位: 四边中点（避开四角分钟点）, 背板沉头→穿格栅柱自攻→压紧前面板
screw_pts = [[face_w / 2, dot_off], [face_w / 2, face_d - dot_off],
             [dot_off, face_d / 2], [face_w - dot_off, face_d / 2]];

// ===== 字母阵（必须与 firmware/wordclock-core/src/lib.rs 一字不差, 含填充字母）=====
LETTERS = [
  ["I","T","L","I","S","A","S","A","M","P","M"],
  ["A","C","Q","U","A","R","T","E","R","D","C"],
  ["T","W","E","N","T","Y","F","I","V","E","X"],
  ["H","A","L","F","S","T","E","N","F","T","O"],
  ["P","A","S","T","E","R","U","N","I","N","E"],
  ["O","N","E","S","I","X","T","H","R","E","E"],
  ["F","O","U","R","F","I","V","E","T","W","O"],
  ["E","I","G","H","T","E","L","E","V","E","N"],
  ["S","E","V","E","N","T","W","E","L","V","E"],
  ["T","E","N","S","E","O","C","L","O","C","K"],
];
letter_size = 10.5;              // 字高（格内净空 ≈ pitch - grid_wall = 15.4）
font_name   = "Allerta Stencil"; // glyph="font" 时使用, 需已安装(Google Fonts)

// 字形方案: "font" = Allerta Stencil(需装字体) | "stroke" = 内置单线字形(无依赖, 无孤岛)
glyph = "font"; // ["font", "stroke"]

// 行号约定与 lib.rs 一致: r=0(ITLIS 行)是正视圈的顶部行 → Y 翻转
// 装配含义: LED 链起点(0 号灯)在左上角, 蛇形向下
// 基线对齐: 大写字母字身底边对齐(Q 的尾巴正确下垂), 不用 center(会被降部字母抬离格心)
cap_h_ratio = 0.72; // Allerta 的大写高度/字号比, 微调用

module letters_2d() {
    for (r = [0:rows - 1]) for (c = [0:cols - 1])
        translate([(c + 0.5) * pitch,
                   grid_d - (r + 0.5) * pitch - letter_size * cap_h_ratio / 2])
            if (glyph == "font")
                text(LETTERS[r][c], size = letter_size, font = font_name,
                     halign = "center", valign = "baseline");
            else
                translate([-32 * ss_sc, -50 * ss_sc]) scale(ss_sc)
                    ss_letter(LETTERS[r][c]); // 字形坐标中心(32,50)对齐格心
}

// ===== 内置单线字形(stroke font) =====
// 无孤岛设计: A/D/O/P/Q/R 的封闭区用"桥接缺口"切开(经典 stencil 处理)。
// 坐标系: cap 高=100, 基准宽 64(中心 x=32), y 向上; ss_sc 控制实际字高。
// 想改风格: 调 ss_r(笔画粗细)/ss_sc(字号)/ss_gap(缺口宽), 或改折线坐标。
ss_r    = 10;     // 笔画半径 → 笔画宽 2.1mm @ ss_sc=0.105
ss_sc   = 0.105;  // cap 100 → 10.5mm 字高
ss_gap  = 10;     // 桥接缺口宽(≈1.05mm, 兼顾打印与视觉)

module _seg(p) { // 折线 → 圆头胶囊笔画
    for (i = [0:len(p) - 2])
        hull() {
            translate(p[i])     circle(r = ss_r, $fn = 20);
            translate(p[i + 1]) circle(r = ss_r, $fn = 20);
        }
}
module _cut(b) translate([b[0], b[1]]) square([b[2] - b[0], b[3] - b[1]]);

// 每字母的笔画折线(多段)
function ss_polys(ch) =
    ch == "A" ? [[[0,0],[32,100]], [[64,0],[32,100]], [[12,34],[52,34]]] :
    ch == "B" ? [[[8,0],[8,100]], [[8,96],[38,96],[54,80],[54,64],[38,52],[8,52]],
                 [[8,48],[40,48],[58,32],[58,16],[44,0],[8,0]]] :
    ch == "C" ? [[[58,78],[44,96],[22,96],[6,80],[6,20],[22,4],[44,4],[58,22]]] :
    ch == "D" ? [[[8,0],[8,100]], [[8,94],[34,94],[52,80],[58,58],[58,42],[52,20],[34,6],[8,6]]] :
    ch == "E" ? [[[56,100],[8,100],[8,0],[56,0]], [[8,50],[48,50]]] :
    ch == "F" ? [[[56,100],[8,100],[8,0]], [[8,52],[46,52]]] :
    ch == "G" ? [[[58,78],[44,96],[22,96],[6,80],[6,20],[22,4],[44,4],[58,22]],
                 [[58,52],[58,26],[38,26]]] :
    ch == "H" ? [[[8,0],[8,100]], [[56,0],[56,100]], [[8,50],[56,50]]] :
    ch == "I" ? [[[32,0],[32,100]]] :
    ch == "J" ? [[[52,100],[52,20],[42,4],[24,4],[12,20]]] :
    ch == "K" ? [[[8,0],[8,100]], [[56,100],[8,44]], [[24,62],[56,0]]] :
    ch == "L" ? [[[8,100],[8,0],[56,0]]] :
    ch == "M" ? [[[-6,0],[-6,100],[32,24],[70,100],[70,0]]] :
    ch == "N" ? [[[8,0],[8,100],[56,0],[56,100]]] :
    ch == "O" ? [[[22,96],[42,96],[56,80],[60,50],[56,20],[42,4],[22,4],[8,20],[4,50],[8,80],[22,96]]] :
    ch == "P" ? [[[8,0],[8,100]], [[8,96],[36,96],[52,82],[56,64],[52,46],[36,36],[8,36]]] :
    ch == "Q" ? [[[22,96],[42,96],[56,80],[60,50],[56,20],[42,4],[22,4],[8,20],[4,50],[8,80],[22,96]],
                 [[46,16],[68,-6]]] :
    ch == "R" ? [[[8,0],[8,100]], [[8,96],[36,96],[52,82],[56,64],[52,46],[36,36],[8,36]],
                 [[30,36],[62,0]]] :
    ch == "S" ? [[[56,88],[42,98],[22,98],[8,86],[8,72],[20,60],[44,50],[54,38],[54,24],[42,4],[20,6],[8,18]]] :
    ch == "T" ? [[[0,100],[64,100]], [[32,100],[32,0]]] :
    ch == "U" ? [[[8,100],[8,20],[22,4],[42,4],[56,20],[56,100]]] :
    ch == "V" ? [[[0,100],[32,0],[64,100]]] :
    ch == "W" ? [[[-6,100],[10,0],[32,58],[54,0],[70,100]]] :
    ch == "X" ? [[[4,0],[60,100]], [[60,0],[4,100]]] :
    ch == "Y" ? [[[4,100],[32,50],[60,100]], [[32,50],[32,0]]] :
    ch == "Z" ? [[[4,100],[60,100],[4,0],[60,0]]] : [];

// 桥接缺口矩形 [x1,y1,x2,y2]: 切开封闭区, 同时把笔画断出 stencil 缺口
function ss_bridges(ch) =
    ch == "A" ? [[27, 22, 27 + ss_gap, 46]] :
    ch == "B" ? [[16,44,28,60], [18,-8,30,8]] :
    ch == "D" ? [[16,-2,28,14]] :
    ch == "O" ? [[-2,2,22,24]] :
    ch == "P" ? [[14,26,26,42]] :
    ch == "Q" ? [[-2,2,22,24]] :
    ch == "R" ? [[14,26,26,42]] : [];

module ss_letter(ch) {
    difference() {
        union() for (p = ss_polys(ch)) _seg(p);
        for (b = ss_bridges(ch)) _cut(b);
    }
}

// ===== 格栅 =====
// 纵墙(分隔同行相邻字母)底部开过条缺口 —— 灯条 PCB 整条从缺口下方穿过,
// 这是灯条平铺方案的必然结构; 横墙(分隔行)落在行间隙上, 与灯条不相交, 整根落地。
// 打印方向: 字面朝下(模型翻转), 缺口开口朝上, 无需支撑。
module grid_walls(rows_n = rows) {
    for (i = [0:cols])
        translate([i * pitch - grid_wall / 2, 0, 0])
            difference() {
                cube([grid_wall, rows_n * pitch, grid_h]);
                for (j = [0:rows_n - 1])
                    translate([-0.01, j * pitch + (pitch - notch_w) / 2, -0.01])
                        cube([grid_wall + 0.02, notch_w, notch_h]);
            }
    for (j = [0:rows_n])
        translate([0, j * pitch - grid_wall / 2, 0])
            cube([grid_w, grid_wall, grid_h]);
}

// 四边中点螺丝柱: 与格栅同高, 支撑前面板 + 供 M3 自攻锁紧三明治
module posts() {
    for (p = screw_pts) {
        gx = p[0] - margin;
        gy = p[1] - margin;
        translate([gx - post_d / 2, gy - post_d / 2, 0])
            difference() {
                cube([post_d, post_d, grid_h]);
                translate([post_d / 2, post_d / 2, -0.1])
                    cylinder(d = pilot_d, h = grid_h + 0.2, $fn = 24);
            }
    }
}

module grid() { grid_walls(); posts(); }

// ===== 背板: LED 行槽 + 走线孔 + 夹紧螺丝沉头孔 =====
module backplate() {
    difference() {
        cube([face_w, face_d, plate_t]);
        // LED 行槽: y 向由槽定位灯条(槽中心线=格心行), x 从格栅左缘起
        for (j = [0:rows - 1])
            translate([margin, margin + j * pitch + (pitch - strip_w) / 2, plate_t - strip_t])
                cube([grid_w + chan_clear, strip_w, strip_t + 0.01]);
        // 行端走线孔(蛇形行间过线, 开在右端槽外)
        for (j = [0:rows - 1])
            translate([margin + grid_w + chan_clear + 0.2,
                       margin + j * pitch + pitch / 2 - 2, -0.01])
                cube([5, 4, plate_t + 0.02]);
        // 四角分钟点过线孔
        for (x = [dot_off, face_w - dot_off]) for (y = [dot_off, face_d - dot_off])
            translate([x, y, -0.01]) cylinder(d = 5, h = plate_t + 0.02, $fn = 24);
        // 夹紧螺丝孔: 背板后侧沉头(M3×25 沉头 ×4, 穿格栅柱自攻)
        for (p = screw_pts)
            translate([p[0], p[1], 0]) {
                translate([0, 0, -0.01]) cylinder(d = screw_d, h = plate_t + 0.02, $fn = 24);
                translate([0, 0, -0.01]) cylinder(d1 = 6.6, d2 = screw_d, h = 1.7, $fn = 24);
            }
    }
}

// ===== 前面板: stencil 字母镂空 + 四角分钟点圆窗 + 螺丝过孔 =====
module faceplate() {
    difference() {
        cube([face_w, face_d, plate_t]);
        translate([margin, margin, -0.01])
            linear_extrude(plate_t + 0.02) letters_2d();
        for (x = [dot_off, face_w - dot_off]) for (y = [dot_off, face_d - dot_off])
            translate([x, y, -0.01]) cylinder(d = dot_win, h = plate_t + 0.02, $fn = 48);
        for (p = screw_pts)
            translate([p[0], p[1], -0.01]) cylinder(d = screw_d, h = plate_t + 0.02, $fn = 24);
    }
}

// ===== 四角分钟点光腔 ×4 =====
// 立式打印, 胶粘到背板四角过线孔正上方; 高度=格栅高, 顶住前面板。
// 分钟点的匀光: 腔顶开放, 靠前面板圆窗 + 贴在窗后的硫酸纸小圆片(装配时粘)。
module corner_ring() {
    difference() {
        union() {
            cylinder(d = dot_id + 2 * dot_wall, h = grid_h, $fn = 48);
            cylinder(d = dot_id + 2 * dot_wall + 4, h = 0.8, $fn = 48); // 底部粘接翻边
        }
        translate([0, 0, -0.1]) cylinder(d = dot_id, h = grid_h + 0.2, $fn = 48);
    }
}

// ===== 间距校验规（先打这个!）=====
// 单行 11 格 + 行槽, 与格栅/背板完全同参数。
// 剪一段真实 11 灯灯条嵌入: ① 目视每颗灯是否居中格子
// ② 首末灯与两端挡墙的间隙 → 直接验证 pitch 数值与剪口位置是否正确。
// 纸面推算区分不了 16.58/16.667, 这个 5g 塑料件可以。
module pitch_jig() {
    base_t = 1.6;
    difference() {
        translate([-3, -3, 0]) cube([grid_w + 6, pitch + 6, base_t]);
        translate([0, (pitch - strip_w) / 2, base_t - strip_t])
            cube([grid_w + chan_clear, strip_w, strip_t + 0.01]);
    }
    translate([0, 0, base_t]) grid_walls(1);
}

// ===== 总装预览 =====
if (part == "grid")        grid();
if (part == "backplate")   backplate();
if (part == "faceplate")   faceplate();
if (part == "corner_ring") corner_ring();
if (part == "pitch_jig")   pitch_jig();
if (part == "all") {
    color("grey")  backplate();
    color("black") translate([margin, margin, plate_t]) grid();
    color("white", 0.6) // 扩散层(裁剪件, 不打印): 硫酸纸或 1mm PC 扩散板
        translate([margin, margin, plate_t + grid_h]) cube([grid_w, grid_d, 0.3]);
    color("black") translate([0, 0, plate_t + grid_h + 0.3]) faceplate();
}

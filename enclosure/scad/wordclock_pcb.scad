// =============================================================================
// Word Clock 结构件 v3 — 适配 192×192 LED 灯板 PCB (hardware/pcba.step)
// -----------------------------------------------------------------------------
// 用法: 改 part 选择件, F6 渲染, File > Export > Export as STL
//   part = "shell" / "grid" / "faceplate" / "diffuser" / "all"
//
// 坐标约定 (与灯板一一对应, 便于核对)
//   原点 = 灯板(PCB) 左下角; +X 向右, +Y 向上(正视方向), +Z 朝观察者(出光方向)
//   PCB:        Z = -1.6 … 0            (0 = 灯珠面/正面)
//   格栅:       Z =  0   … 14
//   扩散层:     Z = 14   … 14.5        (裁剪件, 不打印)
//   前面板:     Z = 14.5 … 16.9
//   电子仓:     Z = -16.6 … -1.6       (DevKit 在此)
//   背板:       Z = -19.0 … -16.6
//
// 尺寸来源 (见 docs/pcba-heights.md)
//   PCB 192×192×1.6, pitch 15.0, 灯阵 11×10 居中(GX=13.5, GY=21)
//   四角分钟点 距边 7mm;  安装孔 4×M3 Ø3.2 在四边中点(距边 6mm)
//   元件正面最大高度: C117(1000µF) 11.5mm > 排针 8.5mm > C116 2.5mm > 灯珠 1.6mm
// =============================================================================

part = "all"; // ["shell", "grid", "faceplate", "diffuser", "snap_test", "all", "check"]

// ===== 灯板 / 灯阵 =====
board      = 192.0;
board_t    = 1.6;
pitch      = 15.0;
cols       = 11;
rows       = 10;
gx         = (board - cols * pitch) / 2;   // 13.5
gy         = (board - rows * pitch) / 2;   // 21.0
dot_off    = 7.0;                          // 角点距板边
mount_off  = 6.0;                          // M3 安装孔距板边

// ===== 格栅 (光隔离) =====
grid_h       = 14.0;   // ≥ C117 11.5 + 余量; 也是匀光距离
grid_front_t = 1.5;    // 前端遮光面(盖住控制区元件)
cell_wall    = 1.2;    // 格壁厚 (0.4 喷嘴 ×3)
grid_outer   = 193.0;  // 外廓 (腔体 194 → 0.5 单边间隙)
dot_hole_d   = 9.0;    // 角点光腔内径
dot_block    = 13.0;   // 角点光腔方块边长

// ===== 外壳 =====
shell_outer = 200.0;   // 成品外形 (灯板 192 + 4mm 边)
shell_wall  = 3.0;     // 腔体 = 194
back_t      = 2.4;
bay         = 15.0;    // 电子仓净深 (DevKit + 排线)
ledge_p     = 2.5;     // PCB 托边向腔内伸出的宽度
ledge_t     = 2.8;     // 托边高度(斜底面 42°, 免支撑)

// ===== 前面板 / 卡扣 =====
face_t   = 2.4;
skirt_t  = 2.4;        // 侧裙厚(包住外壳外壁)
skirt_h  = 8.0;        // 侧裙高
skirt_cl = 0.25;       // 侧裙与外壳外壁间隙(滑动配合)
snap_z   = 7.5;        // 卡扣槽下沿(绝对 Z)
snap_h   = 2.8;        // 卡扣高 (上下斜面各 0.95 → 45°)
snap_d   = 0.95;       // 卡扣从裙内面凸出量(槽深 0.6 → 0.1 过盈)
snap_len = 6.0;
dot_win  = 6.0;        // 角点圆窗
diff_t   = 0.5;        // 扩散片厚度

// ===== ESP32-C5-DevKitC-1 (25.40 × 53.60) =====
dev_w    = 25.40;
dev_h    = 53.60;
dev_x    = 12.0;       // 左边缘 X (竖装: 长边竖直, USB 朝下)
dev_clear= 0.20;       // 卡座单边间隙
dev_usb1 = 5.50;       // UART USB-C 中心 (DevKit 局部 X)
dev_usb2 = 19.90;      // 原生 USB-C 中心
dev_usb_w= 10.0;       // 开孔宽
dev_usb_h= 5.0;        // 开孔高
dev_usb_z= -13.8;      // 开孔中心 Z (DevKit 板面 -17.2…-15.6, USB 座在其上 3.3)

// ===== USB-C 供电母口 (面板安装, 参数化) =====
pwr_cx   = 155.0;      // 母口中心 X
pwr_w    = 9.5;        // 开孔宽
pwr_h    = 5.0;        // 开孔高 (含固定耳余量)

// ===== 派生 Z =====
Z_pcb_f = 0;
Z_pcb_b = -board_t;                       // -1.6
Z_bay   = Z_pcb_b - bay;                  // -16.6 背板内面
Z_bk    = Z_bay - back_t;                 // -19.0 背板外面
Z_grid  = grid_h;                         // 14
Z_diff  = Z_grid + diff_t;                // 14.5
Z_face  = Z_diff + face_t;                // 16.9
Z_po    = Z_bay - 0.6;                    // DevKit 卡座底面 -17.2

cx = board / 2;   // 96
cy = board / 2;

echo(str("PCB ", board, "×", board, "  pitch ", pitch,
         "  外形 ", shell_outer, "×", shell_outer,
         "  总厚 ", Z_face - Z_bk, "mm (含面板)"));

// =============================================================================
// 字母阵 —— 必须与 firmware/wordclock-core/src/lib.rs 完全一致
// =============================================================================
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
letter_size = 10.5;
font_name   = "Allerta Stencil";  // Google Fonts, 本机已安装
cap_h_ratio = 0.72;
glyph       = "font";             // ["font", "stroke"]

function cell_cx(c) = gx + (c + 0.5) * pitch;
function cell_cy(r) = board - (gy + (r + 0.5) * pitch);   // r=0 在顶部

module letters_2d() {
    for (r = [0:rows - 1]) for (c = [0:cols - 1])
        translate([cell_cx(c),
                   cell_cy(r) - letter_size * cap_h_ratio / 2])
            if (glyph == "font")
                text(LETTERS[r][c], size = letter_size, font = font_name,
                     halign = "center", valign = "baseline");
            else
                translate([-32 * ss_sc, -50 * ss_sc]) scale(ss_sc)
                    ss_letter(LETTERS[r][c]);
}

// -------------------------- 内置单线字形 (无字体依赖) -------------------------
ss_r = 10; ss_sc = 0.105; ss_gap = 10;
module _seg(p) { for (i = [0:len(p) - 2]) hull() {
    translate(p[i]) circle(r = ss_r, $fn = 20);
    translate(p[i + 1]) circle(r = ss_r, $fn = 20); } }
module _cut(b) translate([b[0], b[1]]) square([b[2] - b[0], b[3] - b[1]]);
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

// =============================================================================
// 2D 辅助
// =============================================================================
module rounded_sq(w, h, r) { offset(r = r) square([w - 2 * r, h - 2 * r], center = true); }

module cell_holes_2d() {
    for (r = [0:rows - 1]) for (c = [0:cols - 1])
        translate([cell_cx(c) - (pitch - cell_wall) / 2,
                   cell_cy(r) - (pitch - cell_wall) / 2])
            square([pitch - cell_wall, pitch - cell_wall]);
}
module dot_holes_2d() {
    for (x = [dot_off, board - dot_off]) for (y = [dot_off, board - dot_off])
        translate([x, y]) circle(d = dot_hole_d, $fn = 48);
}

// =============================================================================
// 格栅 (grid)
//   - 110 个字母格: 通孔, 壁厚 1.2
//   - 4 个角点光腔: Ø9 通孔 + 13×13 方块
//   - 前端 1.5mm 遮光面: 盖住控制区; 文字格与角点处开孔
//   - 后端其余区域掏空 → 控制区元件(最高 11.5mm)的让位腔
//   - 打印: 前面朝下, 无需支撑
// =============================================================================
module grid_walls_2d() {
    difference() {   // 字母区壁网 (外框扩 0.6 使边界壁也是 1.2)
        translate([gx - 0.6, gy - 0.6])
            square([cols * pitch + 1.2, rows * pitch + 1.2]);
        cell_holes_2d();
    }
    difference() {   // 外圈壁
        translate([cx - grid_outer / 2, cy - grid_outer / 2])
            square([grid_outer, grid_outer]);
        translate([cx - grid_outer / 2 + 2, cy - grid_outer / 2 + 2])
            square([grid_outer - 4, grid_outer - 4]);
    }
    for (x = [dot_off, board - dot_off]) for (y = [dot_off, board - dot_off])
        difference() {   // 角点方块
            translate([x - dot_block / 2, y - dot_block / 2])
                square([dot_block, dot_block]);
            translate([x, y]) circle(d = dot_hole_d, $fn = 48);
        }
}

module grid() {
    translate([0, 0, Z_grid - grid_front_t])
        linear_extrude(grid_front_t) difference() {
            translate([cx - grid_outer / 2, cy - grid_outer / 2])
                square([grid_outer, grid_outer]);
            cell_holes_2d();
            dot_holes_2d();
        }
    translate([0, 0, Z_pcb_f])
        linear_extrude(grid_h - grid_front_t + 0.01) grid_walls_2d();
}

// =============================================================================
// 外壳 (shell)
//   背板 + 四壁 + PCB 托边/定位柱 + 卡扣槽 + DevKit 卡座/USB 开孔 + 电源母口
//   打印: 背板朝下, 壁向上, 无需支撑
// =============================================================================
module devkit_clip() {
    // 局部坐标: 从 X=0 向 +X 伸入 DevKit; Z=0 = 背板内面
    // 立柱 1.2 + 压檐 0.8; 压檐顶面 45° 斜面 (免支撑)
    cube([1.2, 3.6, 1.8]);
    hull() {
        translate([1.2, 0, 1.0]) cube([0.8, 3.6, 0.01]);
        translate([1.2, 0, 1.79]) cube([0.01, 3.6, 0.01]);
    }
}
module devkit_clips() {
    // 左侧 1 + 右侧 1 (Y=47, 避开排针) + 顶边 2 (避开天线区); 底边靠腔壁定位
    translate([dev_x - dev_clear - 1.2, 47, Z_bay]) devkit_clip();
    translate([dev_x + dev_w + dev_clear + 1.2, 47, Z_bay])
        rotate([0, 0, 180]) devkit_clip();
    for (x = [14.0, 35.0])
        translate([x, dev_h - 1 + dev_clear + 1.2, Z_bay])
            rotate([0, 0, 270]) devkit_clip();
}

module shell() {
    L = Z_diff - Z_bk;
    union() {
        difference() {
            // 主体 (背板 + 四壁)
            translate([cx, cy, Z_bk])
                linear_extrude(L) rounded_sq(shell_outer, shell_outer, 4);
            // 腔体
            translate([cx, cy, Z_bay])
                linear_extrude(L + 0.02) rounded_sq(194, 194, 3);
            // 卡扣槽 (外壁一周, 深 0.6; 比卡扣高 0.4 留容差)
            translate([cx, cy, snap_z - 0.2])
                linear_extrude(snap_h + 0.4) difference() {
                    rounded_sq(shell_outer, shell_outer, 4);
                    rounded_sq(shell_outer - 1.2, shell_outer - 1.2, 3.4);
                }
            // DevKit 卡座凹槽 (背板内面下凹 0.6mm)
            translate([dev_x - dev_clear, -1, Z_po - 0.01])
                cube([dev_w + 2 * dev_clear, dev_h + dev_clear + 0.02,
                      0.62]);
            // DevKit 两个 USB-C 开孔 (底壁)
            for (dx = [dev_usb1, dev_usb2])
                translate([dev_x + dx - dev_usb_w / 2,
                           cy - shell_outer / 2 - 1,
                           dev_usb_z - dev_usb_h / 2])
                    cube([dev_usb_w, shell_wall + 2, dev_usb_h]);
            // USB-C 供电母口开孔 (底壁)
            translate([pwr_cx - pwr_w / 2, cy - shell_outer / 2 - 1,
                       dev_usb_z - pwr_h / 2])
                cube([pwr_w, shell_wall + 2, pwr_h]);
        }
        // PCB 托边 (斜底面 ~40°, 免支撑; 顶面 2.5mm 环形支撑面)
        difference() {
            translate([cx, cy, Z_pcb_b - ledge_t])
                linear_extrude(ledge_t + 0.02) rounded_sq(194.04, 194.04, 3);
            hull() {
                translate([cx, cy, Z_pcb_b - ledge_t - 0.1])
                    linear_extrude(0.01) rounded_sq(194.04, 194.04, 3);
                translate([cx, cy, Z_pcb_b + 0.1])
                    linear_extrude(0.01) rounded_sq(189, 189, 1);
            }
        }
        // PCB 定位柱 (穿灯板 4×M3 Ø3.2)
        for (p = [[mount_off, cy], [board - mount_off, cy],
                  [cx, mount_off], [cx, board - mount_off]])
            translate([p[0], p[1], Z_bay])
                cylinder(d = 3.0, h = (Z_pcb_b + 1.0) - Z_bay, $fn = 24);
        devkit_clips();
    }
}

// =============================================================================
// 前面板 (faceplate) — 字母镂空 + 角点圆窗 + 侧裙 + 卡扣
//   打印: 正面朝下 (PEI 纹理), 无需支撑
// =============================================================================
face_outer = shell_outer + 2 * skirt_t;   // 204.8

module snap_hook(len, prot, h) {
    // 凸出 +X, 长 len 沿 Y, 高 h 沿 Z; 根部全高 → 尖端上下各缩 prot (45° 斜面, 免支撑)
    hull() {
        translate([0, -len / 2, 0]) cube([0.01, len, h]);
        translate([prot, -len / 2, prot]) cube([0.01, len, h - 2 * prot]);
    }
}
module hook_at(side, t) {
    // side 0=+X(右) 1=+Y(上) 2=-X(左) 3=-Y(下); t = 沿边偏移; 凸向圆心
    if (side == 0) translate([cx + shell_outer / 2 + skirt_cl, cy + t, snap_z])
        rotate([0, 0, 180]) snap_hook(snap_len, snap_d, snap_h);
    if (side == 1) translate([cx + t, cy + shell_outer / 2 + skirt_cl, snap_z])
        rotate([0, 0, 270]) snap_hook(snap_len, snap_d, snap_h);
    if (side == 2) translate([cx - shell_outer / 2 - skirt_cl, cy + t, snap_z])
        rotate([0, 0, 0]) snap_hook(snap_len, snap_d, snap_h);
    if (side == 3) translate([cx + t, cy - shell_outer / 2 - skirt_cl, snap_z])
        rotate([0, 0, 90]) snap_hook(snap_len, snap_d, snap_h);
}

module faceplate() {
    difference() {
        union() {
            // 前板
            translate([cx, cy, Z_diff])
                linear_extrude(face_t) rounded_sq(face_outer, face_outer, 5);
            // 侧裙 (内面留 skirt_cl 间隙)
            translate([cx, cy, Z_diff - skirt_h])
                linear_extrude(skirt_h + 0.01) difference() {
                    rounded_sq(face_outer, face_outer, 5);
                    rounded_sq(shell_outer + 2 * skirt_cl, shell_outer + 2 * skirt_cl, 3);
                }
            // 卡扣
            for (s = [0:3]) for (t = [-56, 56]) hook_at(s, t);
        }
        // 字母镂空
        translate([0, 0, Z_diff - 0.01])
            linear_extrude(face_t + 0.02) letters_2d();
        // 角点圆窗
        for (x = [dot_off, board - dot_off]) for (y = [dot_off, board - dot_off])
            translate([x, y, Z_diff - 0.01])
                cylinder(d = dot_win, h = face_t + 0.02, $fn = 48);
    }
}

// =============================================================================
// 扩散片 (裁剪件, 不打印; 这里只给 192×192×0.5 的模板)
// =============================================================================
module diffuser() {
    translate([0, 0, Z_grid]) cube([board, board, diff_t]);
}

// =============================================================================
// 对齐校验 (part="check"): 灯珠/元件位置 vs 格栅/面板 开孔
// =============================================================================
module pcb_ref() {
    color("ForestGreen", 0.35)
        translate([0, 0, Z_pcb_b]) cube([board, board, board_t]);
    color("Gold") for (r = [0:rows - 1]) for (c = [0:cols - 1])
        translate([cell_cx(c), cell_cy(r), 0]) cube([5, 5, 1.6], center = true);
    color("Red") for (x = [dot_off, board - dot_off])
        for (y = [dot_off, board - dot_off])
            translate([x, y, 0]) cylinder(d = 5, h = 1.6, center = true, $fn = 32);
    // 控制区元件让位块 (排针 / 1000µF / SMD)
    color("RoyalBlue") {
        translate([24, 180.2, 4.27]) cube([6, 13, 8.54], center = true);
        translate([124, 180.2, 4.27]) cube([6, 13, 8.54], center = true);
        translate([148.5, 180.5, 5.75]) cylinder(d = 10.5, h = 11.5, center = true, $fn = 48);
    }
    // 安装孔
    color("Black") for (p = [[mount_off, cy], [board - mount_off, cy],
                             [cx, mount_off], [cx, board - mount_off]])
        translate([p[0], p[1], -2]) cylinder(d = 3.2, h = 5, $fn = 24);
}

// =============================================================================
// 卡扣试配件 (part="snap_test") — 先打这个验证松紧
//   一段直壁(含卡扣槽) + 一段侧裙(含卡扣), 两件分开摆好一次打出来
//   配得太紧 → 调小 snap_d; 太松/不掉 → 调大 snap_d (每次 0.1mm)
// =============================================================================
module snap_test() {
    sg = 90;          // 段长
    // 壁段 (外表面在 X=0, 卡扣槽 X 0…0.6)
    difference() {
        translate([0, -sg / 2, Z_bk]) cube([3, sg, Z_diff - Z_bk]);
        translate([-0.1, -sg / 2 - 0.1, snap_z - 0.2])
            cube([0.6, sg + 0.2, snap_h + 0.4]);
        translate([-6, -sg / 2 - 0.1, Z_bay]) cube([8, sg + 0.2, 60]);
    }
    // 侧裙段 + 1 个卡扣 (整体右移 12mm 单独打印)
    translate([12, 0, 0]) {
        translate([-2 * skirt_cl - skirt_t, -sg / 2, Z_diff - skirt_h])
            cube([skirt_t, sg, skirt_h + 0.01]);
        translate([-2 * skirt_cl, 0, snap_z]) snap_hook(snap_len, snap_d, snap_h);
    }
}

if (part == "grid")      grid();
if (part == "shell")     shell();
if (part == "faceplate") faceplate();
if (part == "diffuser")  diffuser();
if (part == "snap_test") snap_test();
if (part == "all") {
    color("DimGray")    shell();
    color("Black")      grid();
    color("White", 0.5) diffuser();
    color("Black")      faceplate();
}
if (part == "check") {
    pcb_ref();
    color("Black", 0.45) grid();
    color("Gray", 0.25)  faceplate();
}

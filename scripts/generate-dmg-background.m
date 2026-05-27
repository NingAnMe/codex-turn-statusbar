#import <AppKit/AppKit.h>
#import <Foundation/Foundation.h>

static void drawCentered(NSString *text, CGFloat y, NSDictionary *attributes, CGFloat canvasWidth) {
    NSSize textSize = [text sizeWithAttributes:attributes];
    [text drawAtPoint:NSMakePoint((canvasWidth - textSize.width) / 2.0, y)
       withAttributes:attributes];
}

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        if (argc != 2) {
            fprintf(stderr, "usage: generate-dmg-background <output.png>\n");
            return 2;
        }

        NSString *outputPath = [NSString stringWithUTF8String:argv[1]];
        NSSize canvasSize = NSMakeSize(660, 400);
        NSImage *image = [[NSImage alloc] initWithSize:canvasSize];

        [image lockFocus];

        [[NSColor colorWithCalibratedWhite:0.98 alpha:1.0] setFill];
        NSRectFill(NSMakeRect(0, 0, canvasSize.width, canvasSize.height));

        NSDictionary *hintAttributes = @{
            NSFontAttributeName: [NSFont systemFontOfSize:25 weight:NSFontWeightSemibold],
            NSForegroundColorAttributeName: [NSColor colorWithCalibratedWhite:0.16 alpha:1.0],
        };
        drawCentered(@"拖动到 Applications 完成安装", 320, hintAttributes, canvasSize.width);

        NSDictionary *subhintAttributes = @{
            NSFontAttributeName: [NSFont systemFontOfSize:14 weight:NSFontWeightRegular],
            NSForegroundColorAttributeName: [NSColor colorWithCalibratedWhite:0.38 alpha:1.0],
        };
        drawCentered(@"首次启动会自动配置 Codex notify", 294, subhintAttributes, canvasSize.width);

        NSBezierPath *arrow = [NSBezierPath bezierPath];
        [arrow moveToPoint:NSMakePoint(250, 174)];
        [arrow lineToPoint:NSMakePoint(410, 174)];
        [arrow setLineWidth:6.5];
        [arrow setLineCapStyle:NSLineCapStyleRound];
        [[NSColor colorWithCalibratedWhite:0.04 alpha:0.82] setStroke];
        [arrow stroke];

        NSBezierPath *arrowHead = [NSBezierPath bezierPath];
        [arrowHead moveToPoint:NSMakePoint(422, 174)];
        [arrowHead lineToPoint:NSMakePoint(392, 193)];
        [arrowHead lineToPoint:NSMakePoint(392, 155)];
        [arrowHead closePath];
        [[NSColor colorWithCalibratedWhite:0.04 alpha:0.86] setFill];
        [arrowHead fill];

        [image unlockFocus];

        NSData *tiffData = [image TIFFRepresentation];
        NSBitmapImageRep *representation = [[NSBitmapImageRep alloc] initWithData:tiffData];
        NSData *pngData = [representation representationUsingType:NSBitmapImageFileTypePNG
                                                       properties:@{}];

        NSError *error = nil;
        NSString *directory = [outputPath stringByDeletingLastPathComponent];
        [[NSFileManager defaultManager] createDirectoryAtPath:directory
                                  withIntermediateDirectories:YES
                                                   attributes:nil
                                                        error:&error];
        if (error != nil) {
            fprintf(stderr, "%s\n", [[error localizedDescription] UTF8String]);
            return 1;
        }

        [pngData writeToFile:outputPath options:NSDataWritingAtomic error:&error];
        if (error != nil) {
            fprintf(stderr, "%s\n", [[error localizedDescription] UTF8String]);
            return 1;
        }
    }

    return 0;
}
